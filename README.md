# Core

The **Core** module of the project — responsible for core logic and services.

## 🚀 Getting Started

This project is fully containerized for development. You’ll only need **Docker** and **Docker Compose** installed.

### 🐳 Run in Development Mode (with Hot Reload)

1. Make sure Docker and Docker Compose are installed.
2. Start the development environment:

```bash
docker compose up --build --watch
```

1. Open your browser at [http://development.mairie360.fr](http://development.mairie360.fr) to access the application.

Changes to your code will automatically trigger a refresh or the rebuild of the affected services.

## 🔐 Keycloak single sign-on

Keycloak sign-in is optional. Set these variables on the `core` service to enable
`POST /api/v1/auth/keycloak` (they are empty in `docker-compose.yml`):

| Variable | Role |
| --- | --- |
| `KEYCLOAK_REALM_URL` | Realm URL as Core reaches it, e.g. `http://keycloak:8080/realms/mairie360` |
| `KEYCLOAK_CLIENT_ID` | Core's OIDC client |
| `KEYCLOAK_CLIENT_SECRET` | Secret of that client (confidential client); required for the migration and the administration mirroring below |
| `KEYCLOAK_ISSUER` | Public issuer of the tokens, when it differs from the realm URL (optional) |

A Keycloak user is matched to a Mairie 360 account by the link stored in `user_identities`, or by
its verified e-mail on the first sign-in. No account is created from Keycloak.

### Migrating the existing accounts and roles

Existing accounts must exist in Keycloak before users can sign in through the SSO. The migration
creates them (e-mail as username, no password), carries their roles over as realm roles, disables
archived accounts and records the links. It is safe to run again: nothing is duplicated.

Prerequisites: the client above must be **confidential** with **service accounts enabled**, and its
service account must hold the `realm-management` roles `manage-users`, `view-realm` and
`manage-realm`.

- From the API, as an administrator: `POST /api/v1/admin/keycloak/migration`, optional body
  `{ "send_password_setup_email": true }` to have Keycloak e-mail every newly created account a
  link to set its password (the realm needs SMTP settings). The response lists every account and
  what happened to it.
- From the image, as a one-shot job with the API's environment variables:

```bash
docker run --rm --env-file core.env ghcr.io/mairie360/core-api:<tag> /app/keycloak-migration [--send-password-setup-email]
```

The report is printed as JSON; the exit code is `1` when the run could not complete or at least
one account failed (for example a Keycloak account already linked to another Mairie 360 account).

### Managing users from the administration

Once the client is confidential, the administration endpoints keep the realm in step with what
administrators change in Core (the same `realm-management` roles are needed):

- `POST /api/v1/admin/users/` creates the Keycloak account (or adopts one with the same e-mail),
  asks Keycloak to e-mail it a link to set its password, links it and maps its default role;
- `PATCH /api/v1/admin/users/{id}/` writes the new names / e-mail to the Keycloak account;
- `DELETE /api/v1/admin/users/{id}/` disables the Keycloak account (never deletes it) and ends its
  Keycloak sessions;
- `POST` / `DELETE /api/v1/admin/users/{id}/roles/...` map and unmap the realm role of the same
  name.

Keycloak is called **before** Core writes, and put back as it was when Core refuses the write, so
a failure on either side (`409`, `502`, ...) leaves nothing half done. Accounts that Keycloak does
not know yet (not migrated) are left to the migration; without `KEYCLOAK_CLIENT_SECRET`, only Core
is written.
