-- Test fixtures for the security (ZAP) and performance (k6) stacks, run by the `seeder` service once
-- Liquibase is done.
--
-- User 1 (Admin) is already created by the `create_admin` changeset of the liquibase-migrations
-- image; it is the `sub` of the JWT ZAP injects, so the scan reaches the /api/v1/admin/** routes.
-- User 2 is a plain `User` account, for scans that must run without admin rights, and the member
-- load-test.js adds to its groups.
--
-- User 42, group 3 (owned by user 2, user 42 member) and role 6 (the first custom role, the five
-- base roles being protected from deletion) are the ids of the path parameter examples of the
-- spec: ZAP builds its requests from these examples, so seeding them makes it scan the handlers
-- on real rows instead of stopping at a 404 (or a 403 on a protected role).
--
-- The password is the public argon2id hash of the template admin account: `users.password` only
-- accepts argon2id PHC strings since database 1.3.0 (chk_users_password_hashed).

INSERT INTO users (id, first_name, last_name, email, password, status, is_archived)
VALUES
    (2, 'Test', 'User', 'test.user@mairie360.fr',
     '$argon2id$v=19$m=19456,t=2,p=1$/iKF9PbiDRDs4EKPjlIIhg$UKx9vfwwps250mEP/bYp63CXbEnQGULeUAhDq+az9Aw',
     'active', FALSE),
    (42, 'Jean', 'Dupont', 'jean.dupont.42@mairie360.fr',
     '$argon2id$v=19$m=19456,t=2,p=1$/iKF9PbiDRDs4EKPjlIIhg$UKx9vfwwps250mEP/bYp63CXbEnQGULeUAhDq+az9Aw',
     'active', FALSE)
ON CONFLICT DO NOTHING;

INSERT INTO user_roles (user_id, role_id)
SELECT u.id, r.id FROM roles r CROSS JOIN (VALUES (2), (42)) AS u(id) WHERE r.name = 'User'
ON CONFLICT DO NOTHING;

INSERT INTO groups (id, owner_id, name, description)
VALUES (3, 2, 'Service urbanisme', 'Instruction des permis de construire')
ON CONFLICT DO NOTHING;

-- Guarded: on a database reused from a previous scan, ZAP may have deleted group 3 and taken its
-- name, in which case the group insert above is skipped.
INSERT INTO group_members (group_id, user_id)
SELECT 3, 42 WHERE EXISTS (SELECT 1 FROM groups WHERE id = 3)
ON CONFLICT DO NOTHING;

INSERT INTO roles (id, name, description, can_be_deleted)
VALUES (6, 'agent', 'Agent municipal', TRUE)
ON CONFLICT DO NOTHING;

-- Explicit ids do not advance the SERIAL sequence: move it past the fixtures so the users
-- created during the scan (POST /api/v1/admin/users, /auth/register) do not collide with them.
SELECT setval(pg_get_serial_sequence('users', 'id'), GREATEST((SELECT MAX(id) FROM users), 1));
SELECT setval(pg_get_serial_sequence('groups', 'id'), GREATEST((SELECT MAX(id) FROM groups), 1));
SELECT setval(pg_get_serial_sequence('roles', 'id'), GREATEST((SELECT MAX(id) FROM roles), 1));
