# Journal des releases — Core_API

<!-- release-journal-state {"dev": "090086497aa233ff8b03f5f8043ffdb56734e5e8", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-02 23:06 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `0900864` | 2026-11-02 23:06 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : déconnexion avec le seul refresh token, sans JWT. Répond `204` dans tous les cas (token inconnu, déjà révoqué ou valide), `400` si le corps est mal formé, `429` au-delà de 60 requêtes par minute et par adresse, `503` si la révocation n'a pas pu être publiée sur Redis. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : publie les clés publiques Ed25519 des JWT de session au format JWKS, sans partie privée, avec `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif du compte via `anonymize_user()`, sessions révoquées, compte Keycloak supprimé si le client est confidentiel. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. `404` si l'utilisateur est inconnu, `409` pour l'administrateur initial ou si aucun autre administrateur actif ne peut reprendre ses groupes, projets et événements. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export des données de l'utilisateur tel que renvoyé par `export_user_data()`, identifiants exclus. `404` si l'utilisateur est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` (défaut `all`) : les agents qui n'ont que le rôle Guest reçoivent `403` sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur ; ils gardent leur propre fiche. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tout agent (décision de la mairie). Un compte archivé n'est visible que de lui-même et des administrateurs, sinon `404`. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Authentification JWT : avec `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY`, Core signe les JWT de session en `EdDSA` (`kid`, `iss` `mairie360-core`, `aud` `mairie360`). Sans clé privée, il signe en HS256 tant que `JWT_SECRET` est défini, et les HS256 restent acceptés pendant la transition. Les autres API et les BFF doivent recevoir `JWT_PUBLIC_KEY` pour vérifier ces tokens. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Client généré : `POST /api/v1/auth/logout` est exposé sous `logoutWithRefreshToken()` ; `logout()` reste sur `POST /api/v1/sessions/logout`. Un appelant qui utilisait déjà `logout()` pour le JWT seul n'est pas affecté. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections
- Déconnexion avec JWT expiré : avant, `POST /api/v1/sessions/logout` et `POST /api/v1/sessions/revoke` répondaient `401`, et le refresh token restait valide 30 jours côté serveur. Maintenant, `POST /api/v1/auth/logout` révoque la session à partir du seul refresh token. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Dates de session (`created_at`, `expires_at`, `revoked_at`) sur `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` : avant `2026-09-16 08:42:11 UTC`, que `SessionList` ne sait pas lire (« Date indisponible », bouton de révocation absent). Maintenant ISO 8601 UTC à la seconde, `2026-09-16T08:42:11Z`. Le type reste `string`. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Dernier administrateur : avant, donner un autre rôle au dernier administrateur actif, lui retirer `Admin` ou l'archiver était accepté. Maintenant `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` avec le message `LAST_ADMIN_MESSAGE` et n'écrivent rien. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Rôle `Admin` : avant, `PATCH` ou `PUT /api/v1/admin/roles/{id}` avec `can_be_deleted: true` retirait la protection, après quoi le rôle pouvait être renommé ou supprimé. Maintenant un rôle système (`can_be_deleted = false`) répond `403` avec `System roles stay protected: can_be_deleted cannot be changed.`. Un rôle personnalisé que l'on protège le reste ensuite. `null` ou champ omis : inchangé. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))

#### Technique
- CI : `cicd.yml`, `promote-staging.yml`, `promote-prod.yml` et `gdpr-pr.yml` appellent CICD v5.2.2, avec `ANTHROPIC_WORKSPACE_ID` transmis à `promote-prod` ; test `tests/mair_771_cicd_version.rs`. (MAIR-771, [#357](https://github.com/mairie360/Core_API/pull/357)) Auparavant, trois bumps de CICD v4.4.1 à v4.4.3 pour les releases prod (#325, #327, #330).
- Image : runtime `gcr.io/distroless/cc-debian13`, builder `rust:1.99-slim-trixie`, `development.Dockerfile` en `rust:1.99-trixie`, avec test `tests/dockerfile_test.rs`. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))
- Journaux : la span racine des requêtes n'enregistre plus la query string ni le `Debug` des erreurs ; `mairie360_api_lib` passe de 3.0.0 à 4.2.0 puis 4.3.0. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Base : image Database `dev-76437e3` dans les fichiers compose, qui fournit `anonymize_user()` et `export_user_data()`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Tests et contrôles ajoutés : checklist RGPD des PR (#272), simulation d'usage RGPD (#273), matrice d'accès (#274), tests d'authentification et de révocation (#268), tests de limites de débit des BFF (#342), tests d'identifiants d'opération (#349).
- Dépendances : Renovate met à jour le digest de `rust:1.99-bookworm` (#328, #332, #334) et tokio en 1.54.1 (#333, diff non fourni).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : déconnexion avec le seul refresh token, sans JWT. Répond `204` dans tous les cas (token inconnu, déjà révoqué ou valide), `400` si le corps est mal formé, `429` au-delà de 60 requêtes par minute et par adresse, `503` si la révocation n'a pas pu être publiée sur Redis. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : publie les clés publiques Ed25519 des JWT de session au format JWKS, sans partie privée, avec `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif du compte via `anonymize_user()`, sessions révoquées, compte Keycloak supprimé si le client est confidentiel. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. `404` si l'utilisateur est inconnu, `409` pour l'administrateur initial ou si aucun autre administrateur actif ne peut reprendre ses groupes, projets et événements. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export des données de l'utilisateur tel que renvoyé par `export_user_data()`, identifiants exclus. `404` si l'utilisateur est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` (défaut `all`) : les agents qui n'ont que le rôle Guest reçoivent `403` sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur ; ils gardent leur propre fiche. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tout agent (décision de la mairie). Un compte archivé n'est visible que de lui-même et des administrateurs, sinon `404`. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Authentification JWT : avec `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY`, Core signe les JWT de session en `EdDSA` (`kid`, `iss` `mairie360-core`, `aud` `mairie360`). Sans clé privée, il signe en HS256 tant que `JWT_SECRET` est défini, et les HS256 restent acceptés pendant la transition. Les autres API et les BFF doivent recevoir `JWT_PUBLIC_KEY` pour vérifier ces tokens. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Client généré : `POST /api/v1/auth/logout` est exposé sous `logoutWithRefreshToken()` ; `logout()` reste sur `POST /api/v1/sessions/logout`. Un appelant qui utilisait déjà `logout()` pour le JWT seul n'est pas affecté. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections
- Déconnexion avec JWT expiré : avant, `POST /api/v1/sessions/logout` et `POST /api/v1/sessions/revoke` répondaient `401`, et le refresh token restait valide 30 jours côté serveur. Maintenant, `POST /api/v1/auth/logout` révoque la session à partir du seul refresh token. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Dates de session (`created_at`, `expires_at`, `revoked_at`) sur `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` : avant `2026-09-16 08:42:11 UTC`, que `SessionList` ne sait pas lire (« Date indisponible », bouton de révocation absent). Maintenant ISO 8601 UTC à la seconde, `2026-09-16T08:42:11Z`. Le type reste `string`. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Dernier administrateur : avant, donner un autre rôle au dernier administrateur actif, lui retirer `Admin` ou l'archiver était accepté. Maintenant `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` avec le message `LAST_ADMIN_MESSAGE` et n'écrivent rien. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Rôle `Admin` : avant, `PATCH` ou `PUT /api/v1/admin/roles/{id}` avec `can_be_deleted: true` retirait la protection, après quoi le rôle pouvait être renommé ou supprimé. Maintenant un rôle système (`can_be_deleted = false`) répond `403` avec `System roles stay protected: can_be_deleted cannot be changed.`. Un rôle personnalisé que l'on protège le reste ensuite. `null` ou champ omis : inchangé. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))

#### Technique
- CI : `cicd.yml`, `promote-staging.yml`, `promote-prod.yml` et `gdpr-pr.yml` appellent CICD v5.2.2, avec `ANTHROPIC_WORKSPACE_ID` transmis à `promote-prod` ; test `tests/mair_771_cicd_version.rs`. (MAIR-771, [#357](https://github.com/mairie360/Core_API/pull/357)) Auparavant, trois bumps de CICD v4.4.1 à v4.4.3 pour les releases prod (#325, #327, #330).
- Image : runtime `gcr.io/distroless/cc-debian13`, builder `rust:1.99-slim-trixie`, `development.Dockerfile` en `rust:1.99-trixie`, avec test `tests/dockerfile_test.rs`. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))
- Journaux : la span racine des requêtes n'enregistre plus la query string ni le `Debug` des erreurs ; `mairie360_api_lib` passe de 3.0.0 à 4.2.0 puis 4.3.0. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Base : image Database `dev-76437e3` dans les fichiers compose, qui fournit `anonymize_user()` et `export_user_data()`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Tests et contrôles ajoutés : checklist RGPD des PR (#272), simulation d'usage RGPD (#273), matrice d'accès (#274), tests d'authentification et de révocation (#268), tests de limites de débit des BFF (#342), tests d'identifiants d'opération (#349).
- Dépendances : Renovate met à jour le digest de `rust:1.99-bookworm` (#328, #332, #334) et tokio en 1.54.1 (#333, diff non fourni).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-11-02 23:06 UTC | dev | — | `0900864` | [run](https://github.com/mairie360/Core_API/actions/runs/41539977149) |
| 2026-11-02 17:51 UTC | dev | — | `1843364` | [run](https://github.com/mairie360/Core_API/actions/runs/41493127882) |
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
