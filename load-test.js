// k6 load test, run by ./performance_test.sh (docker-compose-performance.yml).
//
// Built on the shared OpenAPI coverage module (mairie360/CICD `tests/k6/coverage.js`, MAIR-194):
// every operation of the spec served by the API needs exactly one handler ("METHOD /path", path
// as in openapi.json), k6 aborts at init otherwise. When you add an endpoint, add its handler to
// `readHandlers` (GET) or `writeHandlers` (any other method) and send its request through
// `request()` (raw `http.*` calls are not counted).
//
// Load test (MAIR-474) against the data volume of init-perf.sql (10 000 users, 50 000 sessions,
// 2 000 groups), sized by K6_PROFILE (see PROFILES). Two scenarios share the spec, split by HTTP
// method:
// - `reads`: the GET operations, ramping up to the read VUs of the profile, as the Admin, against
//   a group created in setup() and removed in teardown();
// - `writes`: every other operation with the write VUs of the profile. Each handler is
//   self-contained: it creates what it needs through `fixture()` (accounts, roles, groups), sends
//   its request, then deletes what it created, so the handlers do not depend on their order.
//   Deleted accounts are archived by the API (soft delete), the only rows left behind.
// A third scenario, `login_rush`, replays the morning rush: up to LOGIN_RUSH_RATE password logins
// per second (argon2, CPU bound) on accounts created in setup(), alongside the two others.
//
// The authentication flows run end to end on throwaway accounts: admin creates the account → login (412, first
// connection) → force_change_password → login → refresh → revoke, and forgot_password → reset
// through the token the API e-mails to Mailpit (read through its HTTP API).
import http from 'k6/http';
import exec from 'k6/execution';
import { check, fail, sleep } from 'k6';
import crypto from 'k6/crypto';
import encoding from 'k6/encoding';
import { createCoverage, loadSpec } from '/coverage.js';

const BASE_URL = (__ENV.BASE_URL || 'http://localhost:3000').replace(/\/+$/, '');
const MAILPIT_URL = (__ENV.MAILPIT_URL || 'http://localhost:8025').replace(/\/+$/, '');

// Admin JWT (sub=1, the Admin seeded by liquibase, role=admin) signed with the stack's random
// JWT_SECRET by stack_secrets.sh (MAIR-428): no token is committed any more.
const TOKEN = __ENV.JWT;
if (!TOKEN) {
  throw new Error('JWT is not set: run performance_test.sh, which signs the admin token.');
}
const AUTH = { Authorization: `Bearer ${TOKEN}` };

// Plain `User` account seeded by init-test.sql.
const MEMBER_ID = 2;

const FIRST_PASSWORD = 'MotDePasse!123';
const PASSWORD = 'NouveauMotDePasse!123';
const DEVICE = 'k6 load test';

// p(95) latency budget of each family of operations, in ms (reference machine).
const READ_BUDGET_MS = 200;
const WRITE_BUDGET_MS = 500;
const LOGIN_RUSH_BUDGET_MS = 1000;

// Load profile (MAIR-474), K6_PROFILE:
// - `ci` (default): what the CI runner holds with the same strict thresholds. The runner
//   (ubuntu-latest, 4 vCPU) hosts the API, Postgres, Redis and k6 together;
// - `stress`: the high load, run by hand (`K6_PROFILE=stress ./performance_test.sh`) to find
//   the breaking point on a larger machine, not on every push.
const PROFILES = {
  ci: { readVus: 30, writeVus: 4, loginRushRate: 8 },
  stress: { readVus: 100, writeVus: 10, loginRushRate: 20 },
};
const PROFILE = PROFILES[__ENV.K6_PROFILE || 'ci'];
if (!PROFILE) throw new Error(`Unknown K6_PROFILE ${__ENV.K6_PROFILE}: ${Object.keys(PROFILES).join(', ')}`);

// Morning rush: logins per second at the peak, spread over LOGIN_RUSH_ACCOUNTS accounts.
const LOGIN_RUSH_RATE = PROFILE.loginRushRate;
const LOGIN_RUSH_ACCOUNTS = 40;

// Agents seeded by init-perf.sql (perf.agent.<n>@mairie360.fr), 20 per page of the admin list.
const SEEDED_AGENTS = 10000;
const ADMIN_PAGE_SIZE = 20;

const READ_METHODS = ['get', 'head', 'options'];

/** The served spec restricted to the operations whose method passes `keep`. */
function specSubset(spec, keep) {
  const paths = {};
  for (const path of Object.keys(spec.paths)) {
    const kept = {};
    for (const method of Object.keys(spec.paths[path])) {
      if (keep(method)) kept[method] = spec.paths[path][method];
    }
    if (Object.keys(kept).length > 0) paths[path] = kept;
  }
  return Object.assign({}, spec, { paths });
}

let sequence = 0;

/** Suffix unique across VUs, iterations and runs, for e-mails, role and group names. */
function unique() {
  sequence += 1;
  return `${exec.vu.idInTest}-${sequence}-${Date.now() % 100000000}`;
}

/**
 * Raw call for the fixtures of setup(), teardown() and the write handlers, outside the coverage
 * count. Tagged `op: fixture` so it stays out of the per-operation latency thresholds, but it
 * still counts in `http_req_failed` (unless `expected` lists its status). Aborts the handler (or
 * setup) on any other status.
 */
function fixture(method, path, body, { headers = AUTH, expected = [200, 201, 204] } = {}) {
  const res = http.request(
    method,
    `${BASE_URL}${path}`,
    body === undefined ? null : JSON.stringify(body),
    {
      headers: Object.assign({ 'Content-Type': 'application/json' }, headers),
      tags: { op: 'fixture' },
      responseCallback: http.expectedStatuses(...expected),
    },
  );
  if (!expected.includes(res.status)) {
    fail(`fixture ${method} ${path} answered ${res.status}: ${res.body}`);
  }
  return res;
}

function bearer(jwt) {
  return { Authorization: `Bearer ${jwt}` };
}

function deleteUser(userId) {
  fixture('DELETE', `/api/v1/admin/users/${userId}/`);
}

/** Account created by the admin, still in first connection: its login answers 412 with a token. */
function registerAccount(tag) {
  const email = `k6.${tag}.${unique()}@mairie360.fr`;
  // The creation answers the id of the account: no search by e-mail (MAIR-474).
  const id = fixture('POST', '/api/v1/admin/users/', {
    first_name: 'Agent',
    last_name: 'Charge',
    email,
    password: FIRST_PASSWORD,
  }).json('id');
  return { email, id };
}

function firstConnectionToken(account) {
  return fixture(
    'POST',
    '/api/v1/auth/login',
    { email: account.email, password: FIRST_PASSWORD, device_info: DEVICE },
    { expected: [412] },
  ).json('token');
}

/** Account past its first connection, logging in with PASSWORD. */
function activeAccount(tag) {
  const account = registerAccount(tag);
  fixture('POST', '/api/v1/auth/force_change_password', {
    token: firstConnectionToken(account),
    new_password: PASSWORD,
  });
  return account;
}

/** Session of `account`: its JWT and refresh token. */
function login(account) {
  const res = fixture('POST', '/api/v1/auth/login', {
    email: account.email,
    password: PASSWORD,
    device_info: DEVICE,
  });
  return { jwt: res.headers.Authorization.replace(/^Bearer /, ''), refresh: res.json('refresh_token') };
}

/** Password reset token the API e-mailed to `email`, read back from Mailpit. */
function resetToken(email) {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    const search = http.get(`${MAILPIT_URL}/api/v1/search?query=${encodeURIComponent(`to:"${email}"`)}`, {
      tags: { op: 'fixture' },
    });
    const messages = search.status === 200 ? search.json('messages') || [] : [];
    if (messages.length > 0) {
      const text = http.get(`${MAILPIT_URL}/api/v1/message/${messages[0].ID}`, { tags: { op: 'fixture' } }).json('Text');
      const token = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/.exec(text || '');
      if (token) return token[0];
    }
    sleep(0.2);
  }
  return fail(`fixture: no reset e-mail for ${email} in Mailpit`);
}

function createRole(tag) {
  const name = `k6-${tag}-${unique()}`;
  fixture('POST', '/api/v1/admin/roles/', { name, description: 'k6 fixture', can_be_deleted: true });
  const role = fixture('GET', '/api/v1/admin/roles/')
    .json('roles')
    .find((r) => r.name === name);
  if (!role) fail(`fixture: role ${name} not listed`);
  return role.id;
}

function deleteRole(roleId) {
  fixture('DELETE', `/api/v1/admin/roles/${roleId}`);
}

function createGroup(tag) {
  return fixture('POST', '/api/v1/groups/', { name: `k6 ${tag} ${unique()}`, description: 'k6 fixture' }).json('id');
}

function deleteGroup(groupId) {
  fixture('DELETE', `/api/v1/groups/${groupId}/`);
}

function addGroupMember(groupId, userId) {
  fixture('POST', `/api/v1/groups/${groupId}/users/`, { user_id: userId });
}

/** Grants MEMBER_ID a `read` access on the group `groupId` (a resource instance of type `groups`). */
function grantGroupAccess(groupId) {
  fixture('POST', '/api/v1/ressources/add_access', {
    user_id: MEMBER_ID,
    resource_id: groupId,
    ressource_type: 'groups',
    access_type: 'Read',
  });
}

function groupAccesses(groupId) {
  return fixture('POST', `/api/v1/ressources/${groupId}/access?ressource_type=groups`).json('accesses');
}

function revokeAccess(accessId) {
  fixture('POST', '/api/v1/ressources/remove_access', { access_id: accessId });
}


// ---------------------------------------------------------------------------
// Software passkey authenticator (MAIR-505). The passkey operations need a real WebAuthn
// attestation / assertion: an ECDSA P-256 signature over data the server checks. coverage.js runs
// the handlers synchronously, so the asynchronous WebCrypto cannot be used: P-256 is implemented
// here with BigInt (Jacobian coordinates), SHA-256 comes from k6/crypto. The relying party of the
// stack is `localhost` / `http://localhost:3000` (docker-compose-performance.yml): the origin is
// what the authenticator writes in clientDataJSON, not where the request goes.
// ---------------------------------------------------------------------------

const RP_ID = __ENV.WEBAUTHN_RP_ID || 'localhost';
const RP_ORIGIN = __ENV.WEBAUTHN_RP_ORIGIN || 'http://localhost:3000';

const P256 = {
  p: BigInt('0xffffffff00000001000000000000000000000000ffffffffffffffffffffffff'),
  a: BigInt('0xffffffff00000001000000000000000000000000fffffffffffffffffffffffc'),
  b: BigInt('0x5ac635d8aa3a93e7b3ebbd55769886bc651d06b0cc53b0f63bce3c3e27d2604b'),
  n: BigInt('0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551'),
  gx: BigInt('0x6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296'),
  gy: BigInt('0x4fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5'),
};
const ZERO = BigInt(0);
const ONE = BigInt(1);
const TWO = BigInt(2);
const THREE = BigInt(3);

function mod(x, m) {
  const r = x % m;
  return r < ZERO ? r + m : r;
}

function modInverse(x, m) {
  let [a, b] = [mod(x, m), m];
  let [u, v] = [ONE, ZERO];
  while (b !== ZERO) {
    const q = a / b;
    [a, b] = [b, a - q * b];
    [u, v] = [v, u - q * v];
  }
  return mod(u, m);
}

function bytesToBigInt(bytes) {
  let hex = '';
  for (const byte of bytes) hex += byte.toString(16).padStart(2, '0');
  return hex ? BigInt(`0x${hex}`) : ZERO;
}

function bigIntToBytes(value, length) {
  const hex = value.toString(16).padStart(length * 2, '0');
  const bytes = new Uint8Array(length);
  for (let i = 0; i < length; i += 1) bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return bytes;
}

// Jacobian point arithmetic on P-256; `null` is the point at infinity.
function jacobianDouble(point) {
  if (!point || point.y === ZERO) return null;
  const { p, a } = P256;
  const ysq = mod(point.y * point.y, p);
  const s = mod(BigInt(4) * point.x * ysq, p);
  const zsq = mod(point.z * point.z, p);
  const m = mod(THREE * point.x * point.x + a * zsq * zsq, p);
  const nx = mod(m * m - TWO * s, p);
  const ny = mod(m * (s - nx) - BigInt(8) * ysq * ysq, p);
  const nz = mod(TWO * point.y * point.z, p);
  return { x: nx, y: ny, z: nz };
}

function jacobianAdd(p1, p2) {
  if (!p1) return p2;
  if (!p2) return p1;
  const { p } = P256;
  const z1sq = mod(p1.z * p1.z, p);
  const z2sq = mod(p2.z * p2.z, p);
  const u1 = mod(p1.x * z2sq, p);
  const u2 = mod(p2.x * z1sq, p);
  const s1 = mod(p1.y * z2sq * p2.z, p);
  const s2 = mod(p2.y * z1sq * p1.z, p);
  if (u1 === u2) return s1 === s2 ? jacobianDouble(p1) : null;
  const h = mod(u2 - u1, p);
  const r = mod(s2 - s1, p);
  const hsq = mod(h * h, p);
  const hcu = mod(hsq * h, p);
  const u1hsq = mod(u1 * hsq, p);
  const nx = mod(r * r - hcu - TWO * u1hsq, p);
  const ny = mod(r * (u1hsq - nx) - s1 * hcu, p);
  const nz = mod(h * p1.z * p2.z, p);
  return { x: nx, y: ny, z: nz };
}

function scalarMultiply(scalar, point) {
  let result = null;
  let addend = { x: point.x, y: point.y, z: ONE };
  let k = scalar;
  while (k > ZERO) {
    if (k & ONE) result = jacobianAdd(result, addend);
    addend = jacobianDouble(addend);
    k >>= ONE;
  }
  if (!result) return null;
  const { p } = P256;
  const zinv = modInverse(result.z, p);
  const zinv2 = mod(zinv * zinv, p);
  return { x: mod(result.x * zinv2, p), y: mod(result.y * zinv2 * zinv, p) };
}

function randomScalar() {
  for (;;) {
    const k = bytesToBigInt(new Uint8Array(crypto.randomBytes(32)));
    if (k > ZERO && k < P256.n) return k;
  }
}

function sha256(bytes) {
  return new Uint8Array(crypto.sha256(bytes.buffer, 'binary'));
}

function concat(...parts) {
  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}

function utf8(text) {
  return new Uint8Array(encoding.b64decode(encoding.b64encode(text)));
}

function b64url(bytes) {
  return encoding.b64encode(bytes.buffer, 'rawurl');
}

function fromB64url(text) {
  return new Uint8Array(encoding.b64decode(text, 'rawurl'));
}

/** DER INTEGER of a positive big integer. */
function derInteger(value) {
  let bytes = bigIntToBytes(value, 32);
  let start = 0;
  while (start < bytes.length - 1 && bytes[start] === 0) start += 1;
  bytes = bytes.slice(start);
  if (bytes[0] & 0x80) bytes = concat(new Uint8Array([0]), bytes);
  return concat(new Uint8Array([0x02, bytes.length]), bytes);
}

/** ECDSA P-256 / SHA-256 signature of `message`, DER encoded (what WebAuthn expects). */
function ecdsaSign(privateKey, message) {
  const z = bytesToBigInt(sha256(message));
  for (;;) {
    const k = randomScalar();
    const point = scalarMultiply(k, { x: P256.gx, y: P256.gy });
    const r = mod(point.x, P256.n);
    if (r === ZERO) continue;
    const s = mod(modInverse(k, P256.n) * (z + r * privateKey), P256.n);
    if (s === ZERO) continue;
    const body = concat(derInteger(r), derInteger(s));
    return concat(new Uint8Array([0x30, body.length]), body);
  }
}

/** CBOR byte string header for `length` bytes. */
function cborBytesHeader(length) {
  if (length < 24) return new Uint8Array([0x40 | length]);
  if (length < 256) return new Uint8Array([0x58, length]);
  return new Uint8Array([0x59, length >> 8, length & 0xff]);
}

function cborText(text) {
  const bytes = utf8(text);
  return concat(new Uint8Array([0x60 | bytes.length]), bytes);
}

/** A new P-256 passkey of the authenticator: private scalar, public point, credential id. */
function createPasskey() {
  const privateKey = randomScalar();
  const publicKey = scalarMultiply(privateKey, { x: P256.gx, y: P256.gy });
  return {
    privateKey,
    x: bigIntToBytes(publicKey.x, 32),
    y: bigIntToBytes(publicKey.y, 32),
    credentialId: new Uint8Array(crypto.randomBytes(32)),
    counter: 0,
  };
}

// Authenticator data flags: user present, user verified, attested credential data included.
const FLAG_UP = 0x01;
const FLAG_UV = 0x04;
const FLAG_AT = 0x40;

function counterBytes(counter) {
  return new Uint8Array([(counter >>> 24) & 0xff, (counter >>> 16) & 0xff, (counter >>> 8) & 0xff, counter & 0xff]);
}

/** `clientDataJSON` of a ceremony, as a browser builds it for RP_ORIGIN. */
function clientDataJson(type, challenge) {
  return utf8(JSON.stringify({ type, challenge, origin: RP_ORIGIN, crossOrigin: false }));
}

/**
 * Answers the creation options of `POST /api/v1/user/me/passkeys/options` (`fmt: none`
 * attestation, COSE ES256 key), like `navigator.credentials.create().toJSON()`.
 */
function attest(passkey, options) {
  const publicKey = options.publicKey;
  passkey.counter += 1;
  // COSE_Key: {1: 2 (EC2), 3: -7 (ES256), -1: 1 (P-256), -2: x, -3: y}.
  const coseKey = concat(
    new Uint8Array([0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20]),
    passkey.x,
    new Uint8Array([0x22, 0x58, 0x20]),
    passkey.y,
  );
  const authData = concat(
    sha256(utf8(publicKey.rp.id || RP_ID)),
    new Uint8Array([FLAG_UP | FLAG_UV | FLAG_AT]),
    counterBytes(passkey.counter),
    new Uint8Array(16), // AAGUID: none
    new Uint8Array([passkey.credentialId.length >> 8, passkey.credentialId.length & 0xff]),
    passkey.credentialId,
    coseKey,
  );
  // {"fmt": "none", "attStmt": {}, "authData": <bytes>}
  const attestationObject = concat(
    new Uint8Array([0xa3]),
    cborText('fmt'),
    cborText('none'),
    cborText('attStmt'),
    new Uint8Array([0xa0]),
    cborText('authData'),
    cborBytesHeader(authData.length),
    authData,
  );
  return {
    id: b64url(passkey.credentialId),
    rawId: b64url(passkey.credentialId),
    type: 'public-key',
    response: {
      attestationObject: b64url(attestationObject),
      clientDataJSON: b64url(clientDataJson('webauthn.create', publicKey.challenge)),
      transports: ['internal'],
    },
    clientExtensionResults: {},
  };
}

/** Answers the request options of `POST /api/v1/auth/passkey/options`, like `credentials.get()`. */
function assert(passkey, options) {
  const publicKey = options.publicKey;
  passkey.counter += 1;
  const authData = concat(
    sha256(utf8(publicKey.rpId || RP_ID)),
    new Uint8Array([FLAG_UP | FLAG_UV]),
    counterBytes(passkey.counter),
  );
  const clientData = clientDataJson('webauthn.get', publicKey.challenge);
  const signature = ecdsaSign(passkey.privateKey, concat(authData, sha256(clientData)));
  return {
    id: b64url(passkey.credentialId),
    rawId: b64url(passkey.credentialId),
    type: 'public-key',
    response: {
      authenticatorData: b64url(authData),
      clientDataJSON: b64url(clientData),
      signature: b64url(signature),
      userHandle: null,
    },
    clientExtensionResults: {},
  };
}

/** Registers a new passkey for the session `jwt` through the fixture calls; returns its id. */
function registerPasskey(jwt) {
  const headers = bearer(jwt);
  const passkey = createPasskey();
  const options = fixture('POST', '/api/v1/user/me/passkeys/options', undefined, { headers }).json();
  const res = fixture(
    'POST',
    '/api/v1/user/me/passkeys/',
    { challenge_id: options.challenge_id, label: `k6 ${unique()}`, credential: attest(passkey, options.public_key) },
    { headers, expected: [201] },
  );
  return { passkey, id: res.json('id') };
}

function deletePasskey(jwt, passkeyId) {
  fixture('DELETE', `/api/v1/user/me/passkeys/${passkeyId}/`, undefined, { headers: bearer(jwt) });
}

const spec = loadSpec();

const readHandlers = {
  'GET /health': ({ request }) => check(request(), { 'health 200': (r) => r.status === 200 }),
  'GET /ready': ({ request }) => check(request(), { 'ready 200': (r) => r.status === 200 }),

  // Directory and profile.
  // 'martin' matches hundreds of seeded agents: the page is full.
  'GET /api/v1/user/': ({ request }) =>
    check(request({ query: { search: 'martin', limit: 50 } }), {
      'directory 200': (r) => r.status === 200,
      'directory reads the seed': (r) => r.status === 200 && r.json('users').length === 50,
    }),
  'GET /api/v1/user/me/': ({ request }) => check(request(), { 'me 200': (r) => r.status === 200 }),
  'GET /api/v1/user/me/notifications/': ({ request }) =>
    check(request(), { 'notification settings 200': (r) => r.status === 200 }),
  'GET /api/v1/user/me/preferences/': ({ request }) =>
    check(request(), { 'preferences 200': (r) => r.status === 200 }),
  'GET /api/v1/user/me/passkeys/': ({ request }) =>
    check(request(), { 'passkeys 200': (r) => r.status === 200 && Array.isArray(r.json('passkeys')) }),
  'GET /api/v1/user/{id}/': ({ request }) =>
    check(request({ path: { id: MEMBER_ID } }), { 'user 200': (r) => r.status === 200 }),
  'GET /api/v1/roles/': ({ request }) => check(request(), { 'roles 200': (r) => r.status === 200 }),

  // Sessions of the caller (the static JWT has none: empty lists).
  'GET /api/v1/sessions/': ({ request }) => check(request(), { 'sessions 200': (r) => r.status === 200 }),
  'GET /api/v1/sessions/history': ({ request }) =>
    check(request(), { 'session history 200': (r) => r.status === 200 }),

  // Groups.
  // The Admin is a member of 300 seeded groups.
  'GET /api/v1/groups/': ({ request }) =>
    check(request(), {
      'groups 200': (r) => r.status === 200,
      'groups reads the seed': (r) => r.status === 200 && r.json('groups').length > 0,
    }),
  'GET /api/v1/groups/{group_id}/': ({ request, data }) =>
    check(request({ path: { group_id: data.groupId } }), { 'group 200': (r) => r.status === 200 }),
  'GET /api/v1/groups/{group_id}/users/': ({ request, data }) =>
    check(request({ path: { group_id: data.groupId } }), { 'group users 200': (r) => r.status === 200 }),

  // Administration.
  // Any page of the 10 000 agents, the last ones included (deep OFFSET).
  'GET /api/v1/admin/users/': ({ request }) => {
    const page = 1 + Math.floor(Math.random() * (SEEDED_AGENTS / ADMIN_PAGE_SIZE));
    check(request({ query: { page, page_size: ADMIN_PAGE_SIZE } }), {
      'admin users 200': (r) => r.status === 200,
      'admin users reads the seed': (r) =>
        r.status === 200 && r.json('total') >= SEEDED_AGENTS && r.json('users').length > 0,
    });
  },
  'GET /api/v1/admin/users/{userId}/': ({ request }) =>
    check(request({ path: { userId: MEMBER_ID } }), { 'admin user 200': (r) => r.status === 200 }),
  'GET /api/v1/admin/roles/': ({ request }) => check(request(), { 'admin roles 200': (r) => r.status === 200 }),
};

const writeHandlers = {

  // Authentication: account created by an admin → login (412) → force_change_password → login → refresh → revoke.
  'POST /api/v1/auth/force_change_password': ({ request }) => {
    const account = registerAccount('force');
    check(request({ body: { token: firstConnectionToken(account), new_password: PASSWORD } }), {
      'force change password 200': (r) => r.status === 200,
    });
    deleteUser(account.id);
  },
  'POST /api/v1/auth/login': ({ request }) => {
    const account = activeAccount('login');
    const res = request({ body: { email: account.email, password: PASSWORD, device_info: DEVICE } });
    check(res, { 'login 200': (r) => r.status === 200 && !!r.headers.Authorization });
    if (res.status === 200) {
      fixture('POST', '/api/v1/sessions/revoke', { refresh_token: res.json('refresh_token') }, {
        headers: { Authorization: res.headers.Authorization },
      });
    }
    deleteUser(account.id);
  },
  'POST /api/v1/sessions/refresh': ({ request }) => {
    const account = activeAccount('refresh');
    const session = login(account);
    const res = request({ body: { refresh_token: session.refresh }, headers: bearer(session.jwt) });
    check(res, {
      'refresh 200': (r) => r.status === 200 && !!r.headers.Authorization && !!r.json('refresh_token'),
    });
    // The refresh token is rotated (MAIR-390): only the returned one still revokes the session.
    if (res.status === 200) {
      fixture('POST', '/api/v1/sessions/revoke', { refresh_token: res.json('refresh_token') }, {
        headers: { Authorization: res.headers.Authorization },
      });
    }
    deleteUser(account.id);
  },
  'POST /api/v1/sessions/logout': ({ request }) => {
    const account = activeAccount('logout');
    const session = login(account);
    check(request({ headers: bearer(session.jwt) }), { 'logout 204': (r) => r.status === 204 });
    deleteUser(account.id);
  },
  'POST /api/v1/sessions/revoke': ({ request }) => {
    const account = activeAccount('revoke');
    const session = login(account);
    check(request({ body: { refresh_token: session.refresh }, headers: bearer(session.jwt) }), {
      'revoke 200': (r) => r.status === 200,
    });
    deleteUser(account.id);
  },

  // Passkeys (MAIR-505): the account registers one with its JWT, then signs in with it.
  'POST /api/v1/user/me/passkeys/options': ({ request }) => {
    const account = activeAccount('passkey-options');
    const session = login(account);
    check(request({ headers: bearer(session.jwt) }), {
      'passkey registration options 200': (r) => r.status === 200 && !!r.json('challenge_id'),
    });
    deleteUser(account.id);
  },
  'POST /api/v1/user/me/passkeys/': ({ request }) => {
    const account = activeAccount('passkey-register');
    const session = login(account);
    const headers = bearer(session.jwt);
    const passkey = createPasskey();
    const options = fixture('POST', '/api/v1/user/me/passkeys/options', undefined, { headers }).json();
    check(
      request({
        headers,
        body: { challenge_id: options.challenge_id, label: 'k6 register', credential: attest(passkey, options.public_key) },
      }),
      { 'register passkey 201': (r) => r.status === 201 && !!r.json('id') },
    );
    deleteUser(account.id);
  },
  'DELETE /api/v1/user/me/passkeys/{id}/': ({ request }) => {
    const account = activeAccount('passkey-delete');
    const session = login(account);
    const registered = registerPasskey(session.jwt);
    check(request({ path: { id: registered.id }, headers: bearer(session.jwt) }), {
      'delete passkey 204': (r) => r.status === 204,
    });
    deleteUser(account.id);
  },
  'POST /api/v1/auth/passkey/options': ({ request }) =>
    check(request(), {
      'passkey sign-in options 200': (r) => r.status === 200 && !!r.json('challenge_id'),
    }),
  'POST /api/v1/auth/passkey': ({ request }) => {
    const account = activeAccount('passkey-login');
    const session = login(account);
    const registered = registerPasskey(session.jwt);
    const options = fixture('POST', '/api/v1/auth/passkey/options').json();
    const res = request({
      body: {
        challenge_id: options.challenge_id,
        credential: assert(registered.passkey, options.public_key),
        device_info: DEVICE,
      },
    });
    check(res, { 'passkey sign-in 200': (r) => r.status === 200 && !!r.headers.Authorization });
    if (res.status === 200) {
      fixture('POST', '/api/v1/sessions/revoke', { refresh_token: res.json('refresh_token') }, {
        headers: { Authorization: res.headers.Authorization },
      });
    }
    deletePasskey(session.jwt, registered.id);
    deleteUser(account.id);
  },

  // Password reset: forgot_password → e-mail (Mailpit) → reset_password.
  'POST /api/v1/auth/forgot_password': ({ request }) => {
    const account = activeAccount('forgot');
    check(request({ body: { email: account.email } }), { 'forgot password 200': (r) => r.status === 200 });
    deleteUser(account.id);
  },
  'POST /api/v1/auth/reset_password': ({ request }) => {
    const account = activeAccount('reset');
    fixture('POST', '/api/v1/auth/forgot_password', { email: account.email });
    const res = request({
      body: { token: resetToken(account.email), new_password: FIRST_PASSWORD, device_info: DEVICE },
    });
    check(res, { 'reset password 200': (r) => r.status === 200 && !!r.headers.Authorization });
    if (res.status === 200) {
      fixture('POST', '/api/v1/sessions/revoke', { refresh_token: res.json('refresh_token') }, {
        headers: { Authorization: res.headers.Authorization },
      });
    }
    deleteUser(account.id);
  },

  // Own profile of the Admin (idempotent value).
  'PATCH /api/v1/user/me/': ({ request }) =>
    check(request({ body: { phone: '06 12 34 56 78', phone_country: 'FR' } }), { 'patch me 200': (r) => r.status === 200 }),
  'PATCH /api/v1/user/me/notifications/': ({ request }) =>
    check(request({ body: { email: true } }), { 'patch notification settings 200': (r) => r.status === 200 }),
  'PATCH /api/v1/user/me/preferences/': ({ request }) =>
    check(request({ body: { theme: 'light' } }), { 'patch preferences 200': (r) => r.status === 200 }),

  // Administration of accounts: create → patch → password → roles → delete.
  'POST /api/v1/admin/users/': ({ request }) => {
    const email = `k6.admin.${unique()}@mairie360.fr`;
    const res = request({
      body: { first_name: 'Agent', last_name: 'Cree', email, password: FIRST_PASSWORD, phone_number: '0612345678', phone_country: 'FR' },
    });
    check(res, {
      'admin create user 201': (r) => r.status === 201,
      'admin create user answers its id': (r) => r.status === 201 && Number.isInteger(r.json('id')),
    });
    if (res.status === 201) deleteUser(res.json('id'));
  },
  'PATCH /api/v1/admin/users/{userId}/': ({ request }) => {
    const account = registerAccount('patch');
    check(request({ path: { userId: account.id }, body: { phone_number: '0798765432', phone_country: 'FR' } }), {
      'admin patch user 200': (r) => r.status === 200,
    });
    deleteUser(account.id);
  },
  'PATCH /api/v1/admin/users/{userId}/password': ({ request }) => {
    const account = registerAccount('password');
    check(request({ path: { userId: account.id }, body: { new_password: PASSWORD } }), {
      'admin reset password 204': (r) => r.status === 204,
    });
    deleteUser(account.id);
  },
  'DELETE /api/v1/admin/users/{userId}/': ({ request }) => {
    const account = registerAccount('delete');
    check(request({ path: { userId: account.id } }), { 'admin delete user 204': (r) => r.status === 204 });
  },
  'POST /api/v1/admin/users/{userId}/roles/': ({ request }) => {
    const account = registerAccount('grant');
    const roleId = createRole('grant');
    check(request({ path: { userId: account.id }, body: { role_id: roleId } }), {
      'grant role 200': (r) => r.status === 200,
    });
    fixture('DELETE', `/api/v1/admin/users/${account.id}/roles/${roleId}`);
    deleteRole(roleId);
    deleteUser(account.id);
  },
  'DELETE /api/v1/admin/users/{userId}/roles/{roleId}': ({ request }) => {
    const account = registerAccount('revoke-role');
    const roleId = createRole('revoke');
    fixture('POST', `/api/v1/admin/users/${account.id}/roles/`, { role_id: roleId });
    check(request({ path: { userId: account.id, roleId } }), { 'revoke role 204': (r) => r.status === 204 });
    deleteRole(roleId);
    deleteUser(account.id);
  },

  // Administration of roles: create → put → patch → delete.
  'POST /api/v1/admin/roles/': ({ request }) => {
    const name = `k6-create-${unique()}`;
    const res = request({ body: { name, description: 'Agent municipal', can_be_deleted: true } });
    check(res, { 'create role 200': (r) => r.status === 200 });
    if (res.status === 200) {
      const role = fixture('GET', '/api/v1/admin/roles/')
        .json('roles')
        .find((r) => r.name === name);
      if (role) deleteRole(role.id);
    }
  },
  'PUT /api/v1/admin/roles/{id}': ({ request }) => {
    const roleId = createRole('put');
    check(
      request({
        path: { id: roleId },
        body: { name: `k6-put-${unique()}`, description: 'Agent habilité à instruire', can_be_deleted: true },
      }),
      { 'put role 200': (r) => r.status === 200 },
    );
    deleteRole(roleId);
  },
  'PATCH /api/v1/admin/roles/{id}': ({ request }) => {
    const roleId = createRole('patch');
    check(request({ path: { id: roleId }, body: { description: 'Agent habilité à instruire' } }), {
      'patch role 200': (r) => r.status === 200,
    });
    deleteRole(roleId);
  },
  'DELETE /api/v1/admin/roles/{id}': ({ request }) => {
    const roleId = createRole('delete');
    check(request({ path: { id: roleId } }), { 'delete role 204': (r) => r.status === 204 });
  },

  // Keycloak is not configured in the test stack: both routes answer 503 (expected, so it does not
  // count in `http_req_failed`). The sign-in itself is covered by tests/keycloak_*.rs.
  'POST /api/v1/auth/keycloak': ({ request }) =>
    check(
      request({
        body: { code: 'k6-code', redirect_uri: 'https://login.mairie360.fr/auth/callback', device_info: DEVICE },
        params: { responseCallback: http.expectedStatuses(503) },
      }),
      { 'keycloak sign-in 503 (not configured)': (r) => r.status === 503 },
    ),
  'POST /api/v1/admin/keycloak/migration': ({ request }) =>
    check(request({ params: { responseCallback: http.expectedStatuses(503) } }), {
      'keycloak migration 503 (not configured)': (r) => r.status === 503,
    }),

  // Accesses of a resource instance (here a group): grant → list → revoke.
  'POST /api/v1/ressources/add_access': ({ request }) => {
    const groupId = createGroup('access-add');
    const res = request({
      body: { user_id: MEMBER_ID, resource_id: groupId, ressource_type: 'groups', access_type: 'Read' },
    });
    check(res, { 'add access 200': (r) => r.status === 200 });
    groupAccesses(groupId).forEach((access) => revokeAccess(access.id));
    deleteGroup(groupId);
  },
  'POST /api/v1/ressources/{id}/access': ({ request }) => {
    const groupId = createGroup('access-list');
    grantGroupAccess(groupId);
    check(request({ path: { id: groupId }, query: { ressource_type: 'groups' } }), {
      'list accesses 200': (r) => r.status === 200,
    });
    groupAccesses(groupId).forEach((access) => revokeAccess(access.id));
    deleteGroup(groupId);
  },
  'POST /api/v1/ressources/remove_access': ({ request }) => {
    const groupId = createGroup('access-remove');
    grantGroupAccess(groupId);
    const accesses = groupAccesses(groupId);
    if (accesses.length === 0) fail(`fixture: no access on group ${groupId}`);
    check(request({ body: { access_id: accesses[0].id } }), { 'remove access 200': (r) => r.status === 200 });
    deleteGroup(groupId);
  },

  // Groups: create → patch → members → delete.
  'POST /api/v1/groups/': ({ request }) => {
    const res = request({ body: { name: `k6 create ${unique()}`, description: 'Instruction des permis' } });
    check(res, { 'create group 200': (r) => r.status === 200 });
    if (res.status === 200) deleteGroup(res.json('id'));
  },
  'PATCH /api/v1/groups/{group_id}/': ({ request }) => {
    const groupId = createGroup('patch');
    check(request({ path: { group_id: groupId }, body: { description: 'Instruction des déclarations' } }), {
      'patch group 200': (r) => r.status === 200,
    });
    deleteGroup(groupId);
  },
  'DELETE /api/v1/groups/{group_id}/': ({ request }) => {
    const groupId = createGroup('delete');
    check(request({ path: { group_id: groupId } }), { 'delete group 204': (r) => r.status === 204 });
  },
  'POST /api/v1/groups/{group_id}/users/': ({ request }) => {
    const groupId = createGroup('add');
    check(request({ path: { group_id: groupId }, body: { user_id: MEMBER_ID } }), {
      'add group member 200': (r) => r.status === 200,
    });
    deleteGroup(groupId);
  },
  'DELETE /api/v1/groups/{group_id}/users/{user_id}/': ({ request }) => {
    const groupId = createGroup('remove');
    addGroupMember(groupId, MEMBER_ID);
    check(request({ path: { group_id: groupId, user_id: MEMBER_ID } }), {
      'remove group member 204': (r) => r.status === 204,
    });
    deleteGroup(groupId);
  },
};

const reads = createCoverage(readHandlers, {
  spec: specSubset(spec, (method) => READ_METHODS.includes(method)),
});
const writes = createCoverage(writeHandlers, {
  spec: specSubset(spec, (method) => !READ_METHODS.includes(method)),
});

/** One `p(95)` threshold per operation (`op` tag) of `coverage`. */
function latencyThresholds(coverage, budgetMs) {
  const thresholds = {};
  for (const operation of coverage.operations) {
    thresholds[`http_req_duration{op:${operation.op}}`] = [`p(95)<${budgetMs}`];
  }
  return thresholds;
}

export const options = {
  // setup() creates the LOGIN_RUSH_ACCOUNTS accounts (four argon2 hashes each).
  setupTimeout: '3m',
  scenarios: {
    reads: {
      executor: 'ramping-vus',
      exec: 'readScenario',
      stages: [
        { duration: '30s', target: Math.ceil(PROFILE.readVus / 2) }, // Ramp up
        { duration: '30s', target: PROFILE.readVus },
        { duration: '2m', target: PROFILE.readVus }, // Hold
        { duration: '20s', target: 0 }, // Ramp down
      ],
    },
    writes: {
      executor: 'constant-vus',
      exec: 'writeScenario',
      vus: PROFILE.writeVus,
      duration: '3m20s',
    },
    login_rush: {
      executor: 'ramping-arrival-rate',
      exec: 'loginRushScenario',
      startRate: 0,
      timeUnit: '1s',
      preAllocatedVUs: 50,
      maxVUs: 200,
      stages: [
        { duration: '30s', target: LOGIN_RUSH_RATE }, // Agents arrive
        { duration: '1m', target: LOGIN_RUSH_RATE }, // Peak
        { duration: '20s', target: 0 },
      ],
      startTime: '1m', // Once the reads reach their peak
    },
  },
  thresholds: {
    ...reads.thresholds, // every operation exercised, no handler error (shared counters)
    ...latencyThresholds(reads, READ_BUDGET_MS),
    ...latencyThresholds(writes, WRITE_BUDGET_MS),
    'http_req_duration{op:login_rush}': [`p(95)<${LOGIN_RUSH_BUDGET_MS}`],
    // The login rush must not be shed: every arrival gets a VU.
    dropped_iterations: ['count==0'],
    // Strict (MAIR-474): one unexpected error, wrong status or missing seeded row fails the run.
    http_req_failed: ['rate==0'],
    // A check can fail on a 2xx (missing header or body field) that http_req_failed accepts.
    checks: ['rate==1'],
  },
};

/** Read fixture: a group with user 2 as member. Login rush: active accounts. */
export function setup() {
  const groupId = createGroup('read');
  addGroupMember(groupId, MEMBER_ID);
  const rushAccounts = [];
  for (let i = 0; i < LOGIN_RUSH_ACCOUNTS; i += 1) {
    rushAccounts.push(activeAccount('rush'));
  }
  return { groupId, rushAccounts };
}

export function teardown(data) {
  deleteGroup(data.groupId);
  for (const account of data.rushAccounts) {
    deleteUser(account.id);
  }
}

export function readScenario(data) {
  reads.run({ headers: AUTH, data });
  sleep(1);
}

export function writeScenario(data) {
  writes.run({ headers: AUTH, data });
  sleep(1);
}

/** One password login of a rush account, outside the coverage count (tagged `login_rush`). */
export function loginRushScenario(data) {
  const account = data.rushAccounts[exec.scenario.iterationInTest % data.rushAccounts.length];
  const res = http.post(
    `${BASE_URL}/api/v1/auth/login`,
    JSON.stringify({ email: account.email, password: PASSWORD, device_info: DEVICE }),
    { headers: { 'Content-Type': 'application/json' }, tags: { op: 'login_rush' } },
  );
  check(res, { 'rush login 200': (r) => r.status === 200 && !!r.headers.Authorization });
}
