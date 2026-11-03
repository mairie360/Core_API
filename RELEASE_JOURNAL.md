# Journal des releases — Core_API

<!-- release-journal-state {"dev": "28ff46a0bc9fbf6411d3d162009327912d42f812", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-03 05:58 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `28ff46a` | 2026-11-03 05:58 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés

- `POST /api/v1/auth/logout` (public) met fin à la session à partir du seul refresh token, sans JWT. Il répond toujours `204`, que le token soit inconnu, déjà révoqué ou valide, et renvoie `429` (budget refresh, 60/min par adresse) ou `503` si la révocation n'a pas pu être publiée. Un JWT expiré ne bloque plus la déconnexion. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) publie les clés publiques Ed25519 des JWT de session au format JWKS, sans partie privée, avec `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) efface définitivement un compte via `anonymize_user()` : identité effacée, appartenances, sessions et événements privés supprimés, groupes, projets et événements transmis à un administrateur actif. Les sessions révoquées sont publiées sur la liste de révocation. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. `404` si le compte est inconnu, `409` si l'effacement est refusé (administrateur de test ou dernier administrateur). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient toutes les données liées à l'utilisateur, identifiants exclus (`export_user_data()`). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` : les agents qui n'ont que le rôle Guest reçoivent `403` sur `GET /api/v1/user/` et sur la fiche des autres utilisateurs, et gardent l'accès à leur propre fiche. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement

- Les dates de session (`created_at`, `expires_at`, `revoked_at`) de `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` passent du format `2026-09-16 08:42:11 UTC` à `2026-09-16T08:42:11Z` (RFC 3339 UTC). Un appelant qui parse l'ancien format doit être adapté. Les champs restent des chaînes. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Les JWT de session sont signés en EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`) dès que `JWT_PRIVATE_KEY` est défini ; sinon Core continue de signer en HS256 avec `JWT_SECRET`, et les HS256 restent acceptés tant que `JWT_SECRET` est défini. Les autres APIs ont besoin de `JWT_PUBLIC_KEY`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` (corps `This user is the last active administrator: give the Admin role to another active user first.`) et n'écrivent rien quand l'opération laisserait aucun administrateur actif. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- `PATCH` et `PUT /api/v1/admin/roles/{id}` répondent `403` (`System roles stay protected: can_be_deleted cannot be changed.`) si `can_be_deleted: true` est envoyé sur un rôle système (`can_be_deleted = false`, par exemple `Admin`). Un rôle personnalisé protégé le reste. `can_be_deleted` omis ou `null` garde la valeur actuelle. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- `PATCH /api/v1/user/me/` avec `email` répond `429` (`Too many requests, please try again later.`, avec `Retry-After`) après 5 vérifications du mot de passe par utilisateur sur 15 minutes, bonnes ou mauvaises. Les modifications sans `email` ne sont pas limitées. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- `GET /api/v1/user/{id}/` affiche désormais le numéro de téléphone à tous les agents (décision de la mairie). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- L'opération OpenAPI de `POST /api/v1/auth/logout` s'appelle `logout_with_refresh_token` (client `logoutWithRefreshToken()`) ; `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- Les journaux de requêtes n'enregistrent plus la query string (par exemple une recherche par e-mail) ni le `Debug` des erreurs, et les messages d'erreur de la lib ne contiennent plus les valeurs (SQLSTATE, table et colonne à la place). (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Corrections

- Avant : les dates de session étaient au format `DateTime::to_string`, que `SessionList` de lib-components ne sait pas lire. Les sessions s'affichaient « Date indisponible » et le bouton de révocation n'apparaissait pas. Maintenant : format ISO 8601 UTC (voir changements de comportement). (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Avant : on pouvait rétrograder, retirer ou archiver le dernier administrateur actif, ce qui laissait l'instance sans administration. Maintenant : `409` et rollback (voir changements de comportement). (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Avant : `Admin` pouvait être désprotégé par `PATCH`/`PUT` avec `can_be_deleted: true`, puis renommé ou supprimé. Maintenant : `403`. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Avant : le mot de passe actuel pouvait être deviné sans limite via la modification de l'e-mail (`PATCH /user/me/`). Maintenant : 5 vérifications par utilisateur sur 15 minutes, puis `429`. La vérification est comptée avant la comparaison du hash. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))

#### Technique

- Les images Docker des stacks de test et de sécurité (Redis, Postgres, curl, WireMock, node, nginx, newman, k6, ZAP) sont épinglées par digest (Renovate). (MAIR-, [#350](https://github.com/mairie360/Core_API/pull/350))
- Image d'exécution passée à distroless Debian 13 et builder à `rust:1.99-slim-trixie` ; un test garde que toutes les images sont sur Debian 13. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))
- mairie360/CICD monté en v5.2.2 dans les workflows, avec `ANTHROPIC_WORKSPACE_ID` transmis en promotion prod ; un test vérifie la cohérence des versions. (MAIR-771, [#357](https://github.com/mairie360/Core_API/pull/357))
- Ajout de la checklist RGPD sur les PR (`gdpr-pr.yml`), de la simulation d'usage RGPD et de la matrice d'accès `access-matrix.yaml` avec son test, et de tests d'authentification et de révocation (MAIR-295, MAIR-497, MAIR-288, MAIR-287 : [#272](https://github.com/mairie360/Core_API/pull/272), [#273](https://github.com/mairie360/Core_API/pull/273), [#274](https://github.com/mairie360/Core_API/pull/274), [#268](https://github.com/mairie360/Core_API/pull/268)).
- `mairie360_api_lib` passé en 4.3.0 ; tests de limitation de débit pour les requêtes relayées par les BFF (MAIR-679, [#342](https://github.com/mairie360/Core_API/pull/342)) ; mises à jour de dépendances (tokio 1.54.1, rust digest) et CICD en v4.4.1 à v4.4.3 (MAIR-612).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés

- `POST /api/v1/auth/logout` (public) met fin à la session à partir du seul refresh token, sans JWT. Il répond toujours `204`, que le token soit inconnu, déjà révoqué ou valide, et renvoie `429` (budget refresh, 60/min par adresse) ou `503` si la révocation n'a pas pu être publiée. Un JWT expiré ne bloque plus la déconnexion. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) publie les clés publiques Ed25519 des JWT de session au format JWKS, sans partie privée, avec `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) efface définitivement un compte via `anonymize_user()` : identité effacée, appartenances, sessions et événements privés supprimés, groupes, projets et événements transmis à un administrateur actif. Les sessions révoquées sont publiées sur la liste de révocation. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. `404` si le compte est inconnu, `409` si l'effacement est refusé (administrateur de test ou dernier administrateur). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient toutes les données liées à l'utilisateur, identifiants exclus (`export_user_data()`). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` : les agents qui n'ont que le rôle Guest reçoivent `403` sur `GET /api/v1/user/` et sur la fiche des autres utilisateurs, et gardent l'accès à leur propre fiche. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement

- Les dates de session (`created_at`, `expires_at`, `revoked_at`) de `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` passent du format `2026-09-16 08:42:11 UTC` à `2026-09-16T08:42:11Z` (RFC 3339 UTC). Un appelant qui parse l'ancien format doit être adapté. Les champs restent des chaînes. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Les JWT de session sont signés en EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`) dès que `JWT_PRIVATE_KEY` est défini ; sinon Core continue de signer en HS256 avec `JWT_SECRET`, et les HS256 restent acceptés tant que `JWT_SECRET` est défini. Les autres APIs ont besoin de `JWT_PUBLIC_KEY`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` (corps `This user is the last active administrator: give the Admin role to another active user first.`) et n'écrivent rien quand l'opération laisserait aucun administrateur actif. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- `PATCH` et `PUT /api/v1/admin/roles/{id}` répondent `403` (`System roles stay protected: can_be_deleted cannot be changed.`) si `can_be_deleted: true` est envoyé sur un rôle système (`can_be_deleted = false`, par exemple `Admin`). Un rôle personnalisé protégé le reste. `can_be_deleted` omis ou `null` garde la valeur actuelle. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- `PATCH /api/v1/user/me/` avec `email` répond `429` (`Too many requests, please try again later.`, avec `Retry-After`) après 5 vérifications du mot de passe par utilisateur sur 15 minutes, bonnes ou mauvaises. Les modifications sans `email` ne sont pas limitées. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- `GET /api/v1/user/{id}/` affiche désormais le numéro de téléphone à tous les agents (décision de la mairie). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- L'opération OpenAPI de `POST /api/v1/auth/logout` s'appelle `logout_with_refresh_token` (client `logoutWithRefreshToken()`) ; `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- Les journaux de requêtes n'enregistrent plus la query string (par exemple une recherche par e-mail) ni le `Debug` des erreurs, et les messages d'erreur de la lib ne contiennent plus les valeurs (SQLSTATE, table et colonne à la place). (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Corrections

- Avant : les dates de session étaient au format `DateTime::to_string`, que `SessionList` de lib-components ne sait pas lire. Les sessions s'affichaient « Date indisponible » et le bouton de révocation n'apparaissait pas. Maintenant : format ISO 8601 UTC (voir changements de comportement). (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Avant : on pouvait rétrograder, retirer ou archiver le dernier administrateur actif, ce qui laissait l'instance sans administration. Maintenant : `409` et rollback (voir changements de comportement). (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Avant : `Admin` pouvait être désprotégé par `PATCH`/`PUT` avec `can_be_deleted: true`, puis renommé ou supprimé. Maintenant : `403`. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Avant : le mot de passe actuel pouvait être deviné sans limite via la modification de l'e-mail (`PATCH /user/me/`). Maintenant : 5 vérifications par utilisateur sur 15 minutes, puis `429`. La vérification est comptée avant la comparaison du hash. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))

#### Technique

- Les images Docker des stacks de test et de sécurité (Redis, Postgres, curl, WireMock, node, nginx, newman, k6, ZAP) sont épinglées par digest (Renovate). (MAIR-, [#350](https://github.com/mairie360/Core_API/pull/350))
- Image d'exécution passée à distroless Debian 13 et builder à `rust:1.99-slim-trixie` ; un test garde que toutes les images sont sur Debian 13. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))
- mairie360/CICD monté en v5.2.2 dans les workflows, avec `ANTHROPIC_WORKSPACE_ID` transmis en promotion prod ; un test vérifie la cohérence des versions. (MAIR-771, [#357](https://github.com/mairie360/Core_API/pull/357))
- Ajout de la checklist RGPD sur les PR (`gdpr-pr.yml`), de la simulation d'usage RGPD et de la matrice d'accès `access-matrix.yaml` avec son test, et de tests d'authentification et de révocation (MAIR-295, MAIR-497, MAIR-288, MAIR-287 : [#272](https://github.com/mairie360/Core_API/pull/272), [#273](https://github.com/mairie360/Core_API/pull/273), [#274](https://github.com/mairie360/Core_API/pull/274), [#268](https://github.com/mairie360/Core_API/pull/268)).
- `mairie360_api_lib` passé en 4.3.0 ; tests de limitation de débit pour les requêtes relayées par les BFF (MAIR-679, [#342](https://github.com/mairie360/Core_API/pull/342)) ; mises à jour de dépendances (tokio 1.54.1, rust digest) et CICD en v4.4.1 à v4.4.3 (MAIR-612).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-11-03 05:58 UTC | dev | — | `28ff46a` | [run](https://github.com/mairie360/Core_API/actions/runs/41585688519) |
| 2026-11-02 23:07 UTC | dev | — | `3b40c2a` | [run](https://github.com/mairie360/Core_API/actions/runs/41540032773) |
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
