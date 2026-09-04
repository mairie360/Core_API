import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 20 }, // Montée en charge à 20 utilisateurs virtuels
    { duration: '1m', target: 20 },  // Maintien
    { duration: '10s', target: 0 },  // Descente
  ],
  thresholds: {
    http_req_duration: ['p(95)<200'], // 95% des requêtes sous 200ms sur la machine étalon
    http_req_failed: ['rate<0.01'],    // Moins de 1% d'erreurs
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  const res = http.get(`${BASE_URL}/health`);
  check(res, {
    'status is 200': (r) => r.status === 200,
  });
  sleep(1);
}
