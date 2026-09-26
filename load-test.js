// k6 load test, run by ./performance_test.sh (docker-compose-performance.yml).
//
// Built on the shared OpenAPI coverage module (mairie360/CICD `tests/k6/coverage.js`, MAIR-194):
// every operation of the spec served by the API needs exactly one handler ("METHOD /path", path
// as in openapi.json), k6 aborts at init otherwise. When you add an endpoint, add its handler to
// `readHandlers` (GET) or `writeHandlers` (any other method) and send its request through
// `request()` (raw `http.*` calls are not counted).
//
// Two scenarios share the spec, split by HTTP method:
// - `reads`: the GET operations under the historical profile (ramp up to 20 VUs), as the Admin,
//   against a group created in setup() and removed in teardown();
// - `writes`: every other operation with 2 VUs. Each handler is self-contained: it creates what it
//   needs through `fixture()` (accounts, roles, groups), sends its request, then deletes what it
//   created, so the handlers do not depend on their order. Deleted accounts are archived by the
//   API (soft delete), the only rows left behind.
//
// The authentication flows run end to end on throwaway accounts: register → login (412, first
// connection) → force_change_password → login → refresh → revoke, and forgot_password → reset
// through the token the API e-mails to Mailpit (read through its HTTP API).
import http from 'k6/http';
import exec from 'k6/execution';
import { check, fail, sleep } from 'k6';
import { createCoverage, loadSpec } from '/coverage.js';

const BASE_URL = (__ENV.BASE_URL || 'http://localhost:3000').replace(/\/+$/, '');
const MAILPIT_URL = (__ENV.MAILPIT_URL || 'http://localhost:8025').replace(/\/+$/, '');

// Static HS256 JWT (sub=1, the Admin seeded by liquibase, role=admin, exp=2100, signed with the
// stack's JWT_SECRET=b"secret"), the same one ZAP injects: it passes AdminMiddleware.
const TOKEN =
  __ENV.JWT ||
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwicm9sZSI6ImFkbWluIiwiZXhwIjo0MTAyNDQ0ODAwfQ.xCeBe_2QxRlXW8WXr3t6F69wbEHA93HbP_7l4OTJwjA';
const AUTH = { Authorization: `Bearer ${TOKEN}` };

// Plain `User` account seeded by init-test.sql.
const MEMBER_ID = 2;

const FIRST_PASSWORD = 'MotDePasse!123';
const PASSWORD = 'NouveauMotDePasse!123';
const DEVICE = 'k6 load test';

// p(95) latency budget of each family of operations, in ms (reference machine).
const READ_BUDGET_MS = 200;
const WRITE_BUDGET_MS = 500;

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

function userIdByEmail(email) {
  const users = fixture('GET', `/api/v1/admin/users/?search=${encodeURIComponent(email)}`).json('users');
  const user = users.find((u) => u.email === email);
  if (!user) fail(`fixture: no user ${email}`);
  return user.id;
}

function deleteUser(userId) {
  fixture('DELETE', `/api/v1/admin/users/${userId}/`);
}

/** Registered account still in first connection: its login answers 412 with a token. */
function registerAccount(tag) {
  const email = `k6.${tag}.${unique()}@mairie360.fr`;
  fixture('POST', '/api/v1/auth/register', {
    first_name: 'Agent',
    last_name: 'Charge',
    email,
    password: FIRST_PASSWORD,
  });
  return { email, id: userIdByEmail(email) };
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
  fixture('POST', `/api/v1/groups/${groupId}/users/`, { group_id: groupId, user_id: userId });
}

const spec = loadSpec();

const readHandlers = {
  'GET /health': ({ request }) => check(request(), { 'health 200': (r) => r.status === 200 }),

  // Directory and profile.
  'GET /api/v1/user/': ({ request }) =>
    check(request({ query: { search: 'test', limit: 50 } }), { 'directory 200': (r) => r.status === 200 }),
  'GET /api/v1/user/me/': ({ request }) => check(request(), { 'me 200': (r) => r.status === 200 }),
  'GET /api/v1/user/{id}/': ({ request }) =>
    check(request({ path: { id: MEMBER_ID } }), { 'user 200': (r) => r.status === 200 }),
  'GET /api/v1/roles/': ({ request }) => check(request(), { 'roles 200': (r) => r.status === 200 }),

  // Sessions of the caller (the static JWT has none: empty lists).
  'GET /api/v1/sessions/': ({ request }) => check(request(), { 'sessions 200': (r) => r.status === 200 }),
  'GET /api/v1/sessions/history': ({ request }) =>
    check(request(), { 'session history 200': (r) => r.status === 200 }),

  // Groups.
  'GET /api/v1/groups/': ({ request }) => check(request(), { 'groups 200': (r) => r.status === 200 }),
  'GET /api/v1/groups/{group_id}/': ({ request, data }) =>
    check(request({ path: { group_id: data.groupId } }), { 'group 200': (r) => r.status === 200 }),
  'GET /api/v1/groups/{group_id}/users/': ({ request, data }) =>
    check(request({ path: { group_id: data.groupId } }), { 'group users 200': (r) => r.status === 200 }),

  // Administration.
  'GET /api/v1/admin/users/': ({ request }) =>
    check(request({ query: { page: 1, page_size: 20 } }), { 'admin users 200': (r) => r.status === 200 }),
  'GET /api/v1/admin/users/{userId}/': ({ request }) =>
    check(request({ path: { userId: MEMBER_ID } }), { 'admin user 200': (r) => r.status === 200 }),
  'GET /api/v1/admin/roles/': ({ request }) => check(request(), { 'admin roles 200': (r) => r.status === 200 }),
};

const writeHandlers = {
  'POST /': ({ request }) => check(request(), { 'hello 200': (r) => r.status === 200 }),

  // Authentication: register → login (412) → force_change_password → login → refresh → revoke.
  'POST /api/v1/auth/register': ({ request }) => {
    const email = `k6.register.${unique()}@mairie360.fr`;
    const res = request({ body: { first_name: 'Agent', last_name: 'Inscrit', email, password: FIRST_PASSWORD } });
    check(res, { 'register 201': (r) => r.status === 201 });
    if (res.status === 201) deleteUser(userIdByEmail(email));
  },
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
    check(request({ body: { refresh_token: session.refresh }, headers: bearer(session.jwt) }), {
      'refresh 200': (r) => r.status === 200 && !!r.headers.Authorization,
    });
    fixture('POST', '/api/v1/sessions/revoke', { refresh_token: session.refresh }, { headers: bearer(session.jwt) });
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
    check(request({ body: { phone: '0612345678' } }), { 'patch me 200': (r) => r.status === 200 }),

  // Administration of accounts: create → patch → password → roles → delete.
  'POST /api/v1/admin/users/': ({ request }) => {
    const email = `k6.admin.${unique()}@mairie360.fr`;
    const res = request({
      body: { first_name: 'Agent', last_name: 'Cree', email, password: FIRST_PASSWORD, phone_number: '0612345678' },
    });
    check(res, { 'admin create user 201': (r) => r.status === 201 });
    if (res.status === 201) deleteUser(userIdByEmail(email));
  },
  'PATCH /api/v1/admin/users/{userId}/': ({ request }) => {
    const account = registerAccount('patch');
    check(request({ path: { userId: account.id }, body: { phone_number: '0798765432' } }), {
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
    check(request({ path: { userId: account.id }, body: { role_id: roleId, user_id: account.id } }), {
      'grant role 200': (r) => r.status === 200,
    });
    fixture('DELETE', `/api/v1/admin/users/${account.id}/roles/${roleId}`);
    deleteRole(roleId);
    deleteUser(account.id);
  },
  'DELETE /api/v1/admin/users/{userId}/roles/{roleId}': ({ request }) => {
    const account = registerAccount('revoke-role');
    const roleId = createRole('revoke');
    fixture('POST', `/api/v1/admin/users/${account.id}/roles/`, { role_id: roleId, user_id: account.id });
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
    check(request({ path: { group_id: groupId }, body: { group_id: groupId, user_id: MEMBER_ID } }), {
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
  scenarios: {
    reads: {
      executor: 'ramping-vus',
      exec: 'readScenario',
      stages: [
        { duration: '30s', target: 20 }, // Ramp up to 20 virtual users
        { duration: '1m', target: 20 }, // Hold
        { duration: '10s', target: 0 }, // Ramp down
      ],
    },
    writes: {
      executor: 'constant-vus',
      exec: 'writeScenario',
      vus: 2,
      duration: '1m40s',
    },
  },
  thresholds: {
    ...reads.thresholds, // every operation exercised, no handler error (shared counters)
    ...latencyThresholds(reads, READ_BUDGET_MS),
    ...latencyThresholds(writes, WRITE_BUDGET_MS),
    http_req_failed: ['rate<0.01'], // Less than 1% errors
  },
};

/** Read fixture: a group with user 2 as member. */
export function setup() {
  const groupId = createGroup('read');
  addGroupMember(groupId, MEMBER_ID);
  return { groupId };
}

export function teardown(data) {
  deleteGroup(data.groupId);
}

export function readScenario(data) {
  reads.run({ headers: AUTH, data });
  sleep(1);
}

export function writeScenario(data) {
  writes.run({ headers: AUTH, data });
  sleep(1);
}
