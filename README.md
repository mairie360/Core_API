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

## 🧪 Integration tests (newman)

The end-to-end scenario lives in `tests/postman/collection.json` (Postman v2.1 collection) with
its variables in `tests/postman/environment.json`. CI replays it with the `postman/newman` image
against the published `dev-<sha>` API image (`IMAGE_REF`), without any Postman account. Locally,
leave `IMAGE_REF` empty and the script builds `core-api:local` from `development.Dockerfile`:

```bash
./integration_test.sh
```

The script starts `docker-compose-integration.yml` (Postgres + Liquibase + Redis + fake Resend + API +
newman), waits for the `newman` service, prints its report and exits with its status (`--bail`
stops at the first failing request). To iterate on the collection against a stack already running
on `localhost:3000`:

```bash
docker run --rm --network host -v "$PWD/tests/postman:/etc/newman:ro" postman/newman:6.1.3-alpine \
  run collection.json --environment environment.json
```

The collection is also importable in the Postman app for manual debugging (`baseUrl` defaults to
`http://localhost:3000`).

## ⚡ Directory cache

The user directory `GET /api/v1/user/` is cached in Redis for 60 seconds, one entry per set of
filters shared by every caller (MAIR-524). Every write through Core that changes what it shows
(account creation, update, archive, role grant/revoke/rename/delete, group creation/deletion,
group member add/remove) invalidates it immediately; a change made directly in the database shows
up after at most 60 seconds. When Redis is unavailable the directory is read from Postgres.

## ✉️ Account invitation

`POST /api/v1/admin/users/` creates the account with a provisional password and e-mails the user the
Resend template `account-invitation` (variables `USER_FIRST_NAME`, `SET_PASSWORD_URL`, `EXPIRE_DAYS`,
optional `MAIRIE_NAME`). The link, `<PUBLIC_URL>/reset-password?token=…`, is valid 7 days and is consumed
by `POST /api/v1/auth/reset_password` like a forgot-password link: the agent picks their password and
gets a session. A failed e-mail never undoes the account; the agent can ask for a reset link on the login
page. The template must be published in Resend before deploying.

Core is the only account system: there is no external identity provider.
