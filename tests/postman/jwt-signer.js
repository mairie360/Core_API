// Test-only signer of the integration stack (MAIR-680). The newman sandbox cannot sign Ed25519, and
// since mairie360_api_lib 6.0.0 Core refuses every HS256 token, so the collection asks this service
// for the JWTs it forges (the seeded Admin, the accounts it creates, an expired or unknown user).
// It signs them exactly like Core: `alg` EdDSA, `kid` = RFC 7638 thumbprint of the public key,
// `iss` mairie360-core, `aud` mairie360, with the run's JWT_PRIVATE_KEY (stack_secrets.sh). It only
// runs inside docker-compose-integration.yml, on the internal network, never in a deployment.
//
// POST /sign with a JSON object of claims answers the signed JWT as text/plain; `iss` and `aud`
// default to Core's.
const http = require('http');
const crypto = require('crypto');

const key = crypto.createPrivateKey(process.env.JWT_PRIVATE_KEY.replace(/\\n/g, '\n'));
const { x } = crypto.createPublicKey(key).export({ format: 'jwk' });
const kid = crypto
  .createHash('sha256')
  .update(JSON.stringify({ crv: 'Ed25519', kty: 'OKP', x }))
  .digest('base64url');
const encode = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');

http
  .createServer((req, res) => {
    if (req.method === 'GET' && req.url === '/health') {
      res.end('OK');
      return;
    }
    if (req.method !== 'POST' || req.url !== '/sign') {
      res.statusCode = 404;
      res.end();
      return;
    }
    let body = '';
    req.on('data', (chunk) => {
      body += chunk;
    });
    req.on('end', () => {
      try {
        const claims = { iss: 'mairie360-core', aud: 'mairie360', ...JSON.parse(body) };
        const input = `${encode({ alg: 'EdDSA', typ: 'JWT', kid })}.${encode(claims)}`;
        const signature = crypto.sign(null, Buffer.from(input), key).toString('base64url');
        res.setHeader('Content-Type', 'text/plain');
        res.end(`${input}.${signature}`);
      } catch (error) {
        res.statusCode = 400;
        res.end(String(error));
      }
    });
  })
  .listen(8080);
