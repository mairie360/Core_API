# Journal des releases — Core_API

<!-- release-journal-state {"dev": "6bc963086b397600e21e48a16241b9ba8e222eff", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-04 07:06 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `6bc9630` | 2026-11-04 07:06 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : termine la session du refresh token seul, sans JWT, et répond `204` dans tous les cas (token inconnu, déjà révoqué ou valide). La session est révoquée et publiée sur la liste de révocation. Limité à 60 requêtes par minute et par adresse. Un JWT expiré ne bloque plus la déconnexion. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : publie les clés publiques Ed25519 des JWT de session au format JWKS, sans la clé privée. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : efface définitivement un compte via `anonymize_user()` : identité effacée, appartenances, sessions et événements privés supprimés, groupes, projets et événements transférés à un administrateur actif. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. `404` pour un utilisateur inconnu, `409` pour l'administrateur d'amorçage ou s'il n'existe pas d'autre administrateur actif. Un second appel répond `already_anonymized: true`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export de toutes les données liées à l'utilisateur, sans les identifiants. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` : ferme l'annuaire aux agents qui n'ont que le rôle Guest. `GET /api/v1/user/` et la fiche d'un autre utilisateur répondent `403` ; leur propre fiche reste lisible. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `PATCH /api/v1/user/me/` avec `email` : chaque vérification du `current_password`, bonne ou mauvaise, compte dans un budget de 5 par utilisateur et par 15 minutes. Au-delà, `429` avec `Retry-After`, sans rien vérifier ni modifier. Une modification sans `email` n'est pas limitée. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` : `created_at`, `expires_at` et `revoked_at` sont au format ISO 8601 UTC (`2026-09-16T08:42:11Z`) au lieu de `2026-09-16 08:42:11 UTC`. Ce sont toujours des chaînes ; un appelant qui parse l'ancien format doit être adapté. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- `GET /api/v1/user/{id}/` : le téléphone d'un agent est désormais visible de tous les agents. Un compte archivé répond `404`, sauf à lui-même et aux administrateurs. (PR [#274](https://github.com/mairie360/Core_API/pull/274))
- `POST /api/v1/admin/users/{userId}/roles/`, `DELETE /api/v1/admin/users/{userId}/roles/{roleId}` et `DELETE /api/v1/admin/users/{userId}/` répondent `409` (« This user is the last active administrator… ») quand l'opération ne laisserait plus aucun administrateur actif ; rien n'est écrit. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- `PATCH` et `PUT /api/v1/admin/roles/{id}` : `can_be_deleted: true` sur un rôle système (`can_be_deleted = false`, par exemple `Admin`) répond `403` (« System roles stay protected: can_be_deleted cannot be changed. »). Un rôle personnalisé protégé reste protégé. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Jetons de session : avec `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY`, connexion, connexion Keycloak, réinitialisation et rafraîchissement émettent des JWT EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`). Sans clé privée, Core signe en HS256 avec `JWT_SECRET`, qui reste accepté tant qu'il est défini. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Nom de client généré : `POST /api/v1/auth/logout` devient `logoutWithRefreshToken()`, alors que `logout()` reste `POST /api/v1/sessions/logout`. Un client généré avant ce changement doit être régénéré. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections
- Dates de session : avant, les dates au format « … UTC » n'étaient pas lues par `SessionList`, qui affichait « Date indisponible » et « État indéterminé », et le bouton de révocation n'apparaissait pas. Maintenant, le format ISO 8601 est lu et la liste s'affiche. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Journaux des requêtes : avant, la query string (dont une adresse e-mail recherchée) et le `Debug` des erreurs (dont le jeton de première connexion) étaient journalisés. Maintenant, la query string n'est plus journalisée et les erreurs le sont par leur type et leur contexte. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- Images Docker épinglées par digest mises à jour par Renovate (`rust`, `redis`, `postgres`, `nginx`, `curl`, `k6`, `newman`, `wiremock`, `zaproxy`, `node`), et image d'exécution `Dockerfile` passée en distroless Debian 13 avec test de garde (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)) ; voir aussi [#363](https://github.com/mairie360/Core_API/pull/363), [#350](https://github.com/mairie360/Core_API/pull/350), [#334](https://github.com/mairie360/Core_API/pull/334), [#332](https://github.com/mairie360/Core_API/pull/332), [#328](https://github.com/mairie360/Core_API/pull/328).
- CI et tests : CICD passé en v5.2.2 avec un test de garde (MAIR-771, [#357](https://github.com/mairie360/Core_API/pull/357)), checklist RGPD sur les PR ([#272](https://github.com/mairie360/Core_API/pull/272)), simulation RGPD ([#273](https://github.com/mairie360/Core_API/pull/273)), tests d'authentification et de révocation ([#268](https://github.com/mairie360/Core_API/pull/268)), matrice d'accès ([#274](https://github.com/mairie360/Core_API/pull/274)).
- Dépendances : `mairie360_api_lib` en 4.3.0 ([#346](https://github.com/mairie360/Core_API/pull/346)), tokio 1.54.1 ([#333](https://github.com/mairie360/Core_API/pull/333)), actix-web et redis ([#335](https://github.com/mairie360/Core_API/pull/335)).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : termine la session du refresh token seul, sans JWT, et répond `204` dans tous les cas (token inconnu, déjà révoqué ou valide). La session est révoquée et publiée sur la liste de révocation. Limité à 60 requêtes par minute et par adresse. Un JWT expiré ne bloque plus la déconnexion. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : publie les clés publiques Ed25519 des JWT de session au format JWKS, sans la clé privée. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : efface définitivement un compte via `anonymize_user()` : identité effacée, appartenances, sessions et événements privés supprimés, groupes, projets et événements transférés à un administrateur actif. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. `404` pour un utilisateur inconnu, `409` pour l'administrateur d'amorçage ou s'il n'existe pas d'autre administrateur actif. Un second appel répond `already_anonymized: true`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export de toutes les données liées à l'utilisateur, sans les identifiants. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` : ferme l'annuaire aux agents qui n'ont que le rôle Guest. `GET /api/v1/user/` et la fiche d'un autre utilisateur répondent `403` ; leur propre fiche reste lisible. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `PATCH /api/v1/user/me/` avec `email` : chaque vérification du `current_password`, bonne ou mauvaise, compte dans un budget de 5 par utilisateur et par 15 minutes. Au-delà, `429` avec `Retry-After`, sans rien vérifier ni modifier. Une modification sans `email` n'est pas limitée. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` : `created_at`, `expires_at` et `revoked_at` sont au format ISO 8601 UTC (`2026-09-16T08:42:11Z`) au lieu de `2026-09-16 08:42:11 UTC`. Ce sont toujours des chaînes ; un appelant qui parse l'ancien format doit être adapté. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- `GET /api/v1/user/{id}/` : le téléphone d'un agent est désormais visible de tous les agents. Un compte archivé répond `404`, sauf à lui-même et aux administrateurs. (PR [#274](https://github.com/mairie360/Core_API/pull/274))
- `POST /api/v1/admin/users/{userId}/roles/`, `DELETE /api/v1/admin/users/{userId}/roles/{roleId}` et `DELETE /api/v1/admin/users/{userId}/` répondent `409` (« This user is the last active administrator… ») quand l'opération ne laisserait plus aucun administrateur actif ; rien n'est écrit. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- `PATCH` et `PUT /api/v1/admin/roles/{id}` : `can_be_deleted: true` sur un rôle système (`can_be_deleted = false`, par exemple `Admin`) répond `403` (« System roles stay protected: can_be_deleted cannot be changed. »). Un rôle personnalisé protégé reste protégé. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Jetons de session : avec `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY`, connexion, connexion Keycloak, réinitialisation et rafraîchissement émettent des JWT EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`). Sans clé privée, Core signe en HS256 avec `JWT_SECRET`, qui reste accepté tant qu'il est défini. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Nom de client généré : `POST /api/v1/auth/logout` devient `logoutWithRefreshToken()`, alors que `logout()` reste `POST /api/v1/sessions/logout`. Un client généré avant ce changement doit être régénéré. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections
- Dates de session : avant, les dates au format « … UTC » n'étaient pas lues par `SessionList`, qui affichait « Date indisponible » et « État indéterminé », et le bouton de révocation n'apparaissait pas. Maintenant, le format ISO 8601 est lu et la liste s'affiche. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Journaux des requêtes : avant, la query string (dont une adresse e-mail recherchée) et le `Debug` des erreurs (dont le jeton de première connexion) étaient journalisés. Maintenant, la query string n'est plus journalisée et les erreurs le sont par leur type et leur contexte. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- Images Docker épinglées par digest mises à jour par Renovate (`rust`, `redis`, `postgres`, `nginx`, `curl`, `k6`, `newman`, `wiremock`, `zaproxy`, `node`), et image d'exécution `Dockerfile` passée en distroless Debian 13 avec test de garde (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)) ; voir aussi [#363](https://github.com/mairie360/Core_API/pull/363), [#350](https://github.com/mairie360/Core_API/pull/350), [#334](https://github.com/mairie360/Core_API/pull/334), [#332](https://github.com/mairie360/Core_API/pull/332), [#328](https://github.com/mairie360/Core_API/pull/328).
- CI et tests : CICD passé en v5.2.2 avec un test de garde (MAIR-771, [#357](https://github.com/mairie360/Core_API/pull/357)), checklist RGPD sur les PR ([#272](https://github.com/mairie360/Core_API/pull/272)), simulation RGPD ([#273](https://github.com/mairie360/Core_API/pull/273)), tests d'authentification et de révocation ([#268](https://github.com/mairie360/Core_API/pull/268)), matrice d'accès ([#274](https://github.com/mairie360/Core_API/pull/274)).
- Dépendances : `mairie360_api_lib` en 4.3.0 ([#346](https://github.com/mairie360/Core_API/pull/346)), tokio 1.54.1 ([#333](https://github.com/mairie360/Core_API/pull/333)), actix-web et redis ([#335](https://github.com/mairie360/Core_API/pull/335)).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-11-04 07:06 UTC | dev | — | `6bc9630` | [run](https://github.com/mairie360/Core_API/actions/runs/41788343275) |
| 2026-11-03 21:33 UTC | dev | — | `ca6521a` | [run](https://github.com/mairie360/Core_API/actions/runs/41719586606) |
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
