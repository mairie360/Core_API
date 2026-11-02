# Journal des releases — Core_API

<!-- release-journal-state {"dev": "7dfccfc8b5b6d43d5fda49a82ba756c921fa1974", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-02 17:23 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `7dfccfc` | 2026-11-02 17:23 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés

- `GET /api/v1/auth/jwks` (route publique) publie les clés publiques Ed25519 des JWT de session au format JWKS, sans partie privée, avec `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/auth/logout` (route publique) révoque la session désignée par son `refresh_token`, sans JWT, et répond toujours 204 ; la session est ajoutée à la liste de révocation Redis. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `POST /api/v1/admin/users/{userId}/erase` (admin) efface définitivement un compte via `anonymize_user()` et publie les sessions révoquées ; `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient les données de l'utilisateur via `export_user_data()`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI ; avec `DIRECTORY_GUEST_ACCESS=none`, `GET /api/v1/user/` et la fiche d'un autre utilisateur répondent 403 aux agents qui n'ont que le rôle Guest. Le téléphone d'un agent est désormais visible à tous les agents sur `GET /api/v1/user/{id}/`. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Pipeline RGPD : simulation d'usage et détection de données personnelles dans les journaux (outils de test, sans effet sur l'API). (MAIR-497, [#273](https://github.com/mairie360/Core_API/pull/273) ; MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Changements de comportement

- Les dates de session (`created_at`, `expires_at`, `revoked_at`) renvoyées par `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` passent du format `2026-09-16 08:42:11 UTC` à ISO 8601 UTC `2026-09-16T08:42:11Z` ; les champs restent des chaînes. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Les JWT de session sont signés en EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`) quand `JWT_PRIVATE_KEY` est défini ; sans lui, Core signe encore en HS256 et accepte HS256 tant que `JWT_SECRET` est défini. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Retirer le rôle Admin au dernier administrateur actif, lui attribuer un autre rôle ou l'archiver répond 409 `LAST_ADMIN_MESSAGE` et ne change rien. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- L'opération OpenAPI du nouveau `POST /api/v1/auth/logout` s'appelle `logout_with_refresh_token` : les clients générés gardent `logout()` pour `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- Le rôle Guest seul ne lit plus le répertoire quand `DIRECTORY_GUEST_ACCESS=none` (403) ; sans cette variable, rien ne change.

#### Corrections

- Dates de session : avant, les dates `… UTC` n'étaient pas lues par `SessionList` (« Date indisponible », bouton de révocation absent) ; maintenant elles sont en ISO 8601. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Déconnexion : avant, elle échouait avec un JWT expiré et le refresh token restait valide ; maintenant `POST /api/v1/auth/logout` révoque la session avec le seul refresh token. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Dernier administrateur : avant, un appel pouvait retirer le dernier administrateur ; maintenant la écriture est annulée (409) et deux demandes concurrentes ne passent pas toutes les deux. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Refus de connexion : avant, les réponses pouvaient révéler l'existence d'un compte ; les quatre cas testés répondent désormais à l'identique (test ajouté). (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268))
- Journaux : avant, les requêtes pouvaient journaliser une adresse e-mail (query string) ou un jeton ; maintenant les valeurs ne sont plus journalisées. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Images : l'image d'exécution passe de Debian 12 à Debian 13 (distroless, `libssl3` corrigée). (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))

#### Technique

- Tests et CI : tests d'intégration RGPD, tests de limitation de débit derrière un BFF, tests de contrat des noms d'opération, garde Dockerfile Debian 13, checklist RGPD sur les PR.
- Dépendances et versions : `mairie360_api_lib` 4.2.0 → 4.3.0, base de données `dev-76437e3`, CICD v4.4.0 → v4.4.3, digests Docker `rust:1.99` mis à jour (Renovate), tokio 1.54.1.

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés

- `GET /api/v1/auth/jwks` (route publique) publie les clés publiques Ed25519 des JWT de session au format JWKS, sans partie privée, avec `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/auth/logout` (route publique) révoque la session désignée par son `refresh_token`, sans JWT, et répond toujours 204 ; la session est ajoutée à la liste de révocation Redis. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `POST /api/v1/admin/users/{userId}/erase` (admin) efface définitivement un compte via `anonymize_user()` et publie les sessions révoquées ; `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient les données de l'utilisateur via `export_user_data()`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI ; avec `DIRECTORY_GUEST_ACCESS=none`, `GET /api/v1/user/` et la fiche d'un autre utilisateur répondent 403 aux agents qui n'ont que le rôle Guest. Le téléphone d'un agent est désormais visible à tous les agents sur `GET /api/v1/user/{id}/`. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Pipeline RGPD : simulation d'usage et détection de données personnelles dans les journaux (outils de test, sans effet sur l'API). (MAIR-497, [#273](https://github.com/mairie360/Core_API/pull/273) ; MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Changements de comportement

- Les dates de session (`created_at`, `expires_at`, `revoked_at`) renvoyées par `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` passent du format `2026-09-16 08:42:11 UTC` à ISO 8601 UTC `2026-09-16T08:42:11Z` ; les champs restent des chaînes. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Les JWT de session sont signés en EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`) quand `JWT_PRIVATE_KEY` est défini ; sans lui, Core signe encore en HS256 et accepte HS256 tant que `JWT_SECRET` est défini. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Retirer le rôle Admin au dernier administrateur actif, lui attribuer un autre rôle ou l'archiver répond 409 `LAST_ADMIN_MESSAGE` et ne change rien. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- L'opération OpenAPI du nouveau `POST /api/v1/auth/logout` s'appelle `logout_with_refresh_token` : les clients générés gardent `logout()` pour `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- Le rôle Guest seul ne lit plus le répertoire quand `DIRECTORY_GUEST_ACCESS=none` (403) ; sans cette variable, rien ne change.

#### Corrections

- Dates de session : avant, les dates `… UTC` n'étaient pas lues par `SessionList` (« Date indisponible », bouton de révocation absent) ; maintenant elles sont en ISO 8601. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Déconnexion : avant, elle échouait avec un JWT expiré et le refresh token restait valide ; maintenant `POST /api/v1/auth/logout` révoque la session avec le seul refresh token. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Dernier administrateur : avant, un appel pouvait retirer le dernier administrateur ; maintenant la écriture est annulée (409) et deux demandes concurrentes ne passent pas toutes les deux. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Refus de connexion : avant, les réponses pouvaient révéler l'existence d'un compte ; les quatre cas testés répondent désormais à l'identique (test ajouté). (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268))
- Journaux : avant, les requêtes pouvaient journaliser une adresse e-mail (query string) ou un jeton ; maintenant les valeurs ne sont plus journalisées. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Images : l'image d'exécution passe de Debian 12 à Debian 13 (distroless, `libssl3` corrigée). (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))

#### Technique

- Tests et CI : tests d'intégration RGPD, tests de limitation de débit derrière un BFF, tests de contrat des noms d'opération, garde Dockerfile Debian 13, checklist RGPD sur les PR.
- Dépendances et versions : `mairie360_api_lib` 4.2.0 → 4.3.0, base de données `dev-76437e3`, CICD v4.4.0 → v4.4.3, digests Docker `rust:1.99` mis à jour (Renovate), tokio 1.54.1.

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-11-02 17:23 UTC | dev | — | `7dfccfc` | [run](https://github.com/mairie360/Core_API/actions/runs/41488250282) |
| 2026-11-02 16:58 UTC | dev | — | `6657fe8` | [run](https://github.com/mairie360/Core_API/actions/runs/41484328354) |
| 2026-11-01 22:33 UTC | dev | — | `72dca5b` | [run](https://github.com/mairie360/Core_API/actions/runs/41356438605) |
| 2026-11-01 21:13 UTC | dev | — | `f309422` | [run](https://github.com/mairie360/Core_API/actions/runs/41349142175) |
| 2026-11-01 20:48 UTC | dev | — | `564bfe6` | [run](https://github.com/mairie360/Core_API/actions/runs/41347708018) |
| 2026-11-01 20:15 UTC | dev | — | `e40686f` | [run](https://github.com/mairie360/Core_API/actions/runs/41344959518) |
| 2026-10-31 01:52 UTC | dev | — | `f3229f1` | [run](https://github.com/mairie360/Core_API/actions/runs/41155994544) |
| 2026-10-22 22:08 UTC | dev | — | `619d1b8` | [run](https://github.com/mairie360/Core_API/actions/runs/39861535514) |
| 2026-10-22 06:11 UTC | dev | — | `0edb5c3` | [run](https://github.com/mairie360/Core_API/actions/runs/39731423596) |
| 2026-10-22 03:49 UTC | dev | — | `ea251a2` | [run](https://github.com/mairie360/Core_API/actions/runs/39717500210) |
| 2026-10-21 09:18 UTC | dev | — | `0d723d4` | [run](https://github.com/mairie360/Core_API/actions/runs/39574704778) |
| 2026-10-21 09:13 UTC | prod | v2.2.0 | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39574731861) |
| 2026-10-21 09:07 UTC | dev | — | `9718990` | [run](https://github.com/mairie360/Core_API/actions/runs/39573304899) |
| 2026-10-21 08:33 UTC | dev | — | `e4b0ee0` | [run](https://github.com/mairie360/Core_API/actions/runs/39568592596) |
| 2026-10-21 03:50 UTC | dev | — | `19adc1e` | [run](https://github.com/mairie360/Core_API/actions/runs/39539859800) |
| 2026-10-21 03:14 UTC | staging | — | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39528369269) |
| 2026-10-21 01:36 UTC | dev | — | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39526428093) |
| 2026-10-20 17:07 UTC | staging | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:57 UTC | dev | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:49 UTC | dev | — | `acf2207` | [run](https://github.com/mairie360/Core_API/actions/runs/39459275998) |
| 2026-10-19 20:10 UTC | dev | — | `30d9bb5` | [run](https://github.com/mairie360/Core_API/actions/runs/39315745705) |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
