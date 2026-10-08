-- Data volume for the performance (k6) stack only, run by the `seeder` service after
-- init-test.sql (MAIR-474).
--
-- Without it the lists of load-test.js read a handful of rows, so a missing index, an N+1 or an
-- OFFSET scan never shows in the p(95). Sized for a large municipality:
--   - 10 000 users (role User), names drawn from common French names so the directory search
--     returns full pages;
--   - 50 000 sessions, 5 per user, all revoked but the last one;
--   - 2 000 groups owned by random users, 20 members each (40 000 memberships);
--   - user 1 (the Admin of the k6 JWT) owner of 50 groups, member of 300 others, with 1 000 past
--     sessions, so `GET /groups/` and `GET /sessions/history` read real pages.
--
-- Accounts reuse the public argon2id hash of init-test.sql: they never log in. Idempotent on a
-- database reused from a previous run (fixed e-mails / group names, ON CONFLICT DO NOTHING).

INSERT INTO users (first_name, last_name, email, password, status, is_archived)
SELECT
    (ARRAY['Marie', 'Jean', 'Pierre', 'Sophie', 'Michel', 'Nathalie', 'Philippe', 'Isabelle',
           'Alain', 'Catherine', 'Nicolas', 'Sandrine', 'Julien', 'Camille', 'Thomas', 'Claire'])
        [1 + n % 16],
    (ARRAY['Martin', 'Bernard', 'Dubois', 'Thomas', 'Robert', 'Richard', 'Petit', 'Durand',
           'Leroy', 'Moreau', 'Simon', 'Laurent', 'Lefebvre', 'Michel', 'Garcia', 'David',
           'Bertrand', 'Roux', 'Vincent', 'Fournier'])[1 + (n / 16) % 20],
    format('perf.agent.%s@mairie360.fr', n),
    '$argon2id$v=19$m=19456,t=2,p=1$/iKF9PbiDRDs4EKPjlIIhg$UKx9vfwwps250mEP/bYp63CXbEnQGULeUAhDq+az9Aw',
    'active',
    FALSE
FROM generate_series(1, 10000) AS n
ON CONFLICT DO NOTHING;

INSERT INTO user_roles (user_id, role_id)
SELECT u.id, r.id
FROM users u
CROSS JOIN roles r
WHERE r.name = 'User' AND u.email LIKE 'perf.agent.%@mairie360.fr'
ON CONFLICT DO NOTHING;

-- Five sessions per user, one hour apart; only the most recent one is still open.
INSERT INTO sessions (user_id, token_hash, device_info, ip_address, created_at, revoked_at)
SELECT
    u.id,
    md5(format('perf-session-%s-%s', u.id, s)),
    'Firefox 141 sur Windows 11',
    '10.20.0.1'::inet,
    now() - make_interval(hours => 5 - s),
    CASE WHEN s < 5 THEN now() - make_interval(hours => 5 - s) + interval '30 minutes' END
FROM users u
CROSS JOIN generate_series(1, 5) AS s
WHERE u.email LIKE 'perf.agent.%@mairie360.fr'
ON CONFLICT DO NOTHING;

-- Past sessions of the Admin: the k6 JWT carries no session, so these only fill its history.
INSERT INTO sessions (user_id, token_hash, device_info, ip_address, created_at, revoked_at)
SELECT
    1,
    md5(format('perf-admin-session-%s', s)),
    'Chrome 140 sur macOS',
    '10.20.0.2'::inet,
    now() - make_interval(hours => s),
    now() - make_interval(hours => s) + interval '30 minutes'
FROM generate_series(1, 1000) AS s
ON CONFLICT DO NOTHING;

-- 2 000 groups (the owner is added as a member by trigger_add_owner_as_member), the first 50
-- owned by the Admin.
INSERT INTO groups (owner_id, name, description)
SELECT
    CASE WHEN g <= 50 THEN 1 ELSE owner.id END,
    format('Perf service %s', g),
    'Service municipal généré pour les tests de charge'
FROM generate_series(1, 2000) AS g
CROSS JOIN LATERAL (
    SELECT id FROM users
    WHERE email = format('perf.agent.%s@mairie360.fr', 1 + (g * 7919) % 10000)
) AS owner
ON CONFLICT DO NOTHING;

-- 20 members per group, spread over the 10 000 agents.
INSERT INTO group_members (group_id, user_id)
SELECT grp.id, member.id
FROM groups grp
CROSS JOIN generate_series(1, 20) AS m
JOIN users member
    ON member.email = format('perf.agent.%s@mairie360.fr', 1 + (grp.id * 31 + m * 499) % 10000)
WHERE grp.name LIKE 'Perf service %'
ON CONFLICT DO NOTHING;

-- The Admin is also a member of 300 groups it does not own.
INSERT INTO group_members (group_id, user_id)
SELECT grp.id, 1
FROM groups grp
WHERE grp.name LIKE 'Perf service %' AND grp.owner_id <> 1
ORDER BY grp.id
LIMIT 300
ON CONFLICT DO NOTHING;

ANALYZE users;
ANALYZE sessions;
ANALYZE groups;
ANALYZE group_members;
ANALYZE user_roles;
