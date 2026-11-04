# Journal des releases — Core_API

<!-- release-journal-state {"dev": "b0a53b8dc02c1d88a8165e19cc003d6acaab58d0", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-04 14:17 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `b0a53b8` | 2026-11-04 14:17 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : déconnexion avec le seul refresh token, sans JWT. Répond `204` (y compris pour un jeton inconnu ou déjà révoqué), `400` si le corps est invalide, `429` (budget refresh, 60/min par adresse) et `503` si la révocation n'a pas pu être publiée sur Redis (réessayer). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : clés publiques Ed25519 des JWT de session au format JWKS, sans clé privée, `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif (art. 17 RGPD) via `anonymize_user()`. `404` si le compte est inconnu, `409` si c'est le compte d'amorçage ou s'il n'existe pas d'autre administrateur actif pour reprendre groupes, projets et événements. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. Les sessions effacées sont publiées sur la liste de révocation. Avec Keycloak, le compte est désactivé puis supprimé (`502` si seule cette dernière étape échoue). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export JSON de toutes les données liées à l'utilisateur, identifiants exclus (art. 15 et 20 RGPD), `404` si le compte est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))

#### Changements de comportement
- Dates de session (`GET /api/v1/sessions/`, `GET /api/v1/sessions/history`, `GET /api/v1/admin/users/{userId}/`) : `created_at`, `expires_at` et `revoked_at` sont au format ISO 8601 UTC (`2026-09-16T08:42:11Z`) au lieu de `2026-09-16 08:42:11 UTC`. Les champs restent des chaînes ; un appelant qui parse l'ancien format doit être adapté. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- JWT de session : si `JWT_PRIVATE_KEY` est défini, Core signe en `EdDSA` (`kid`, `iss` `mairie360-core`, `aud` `mairie360`). Les autres APIs et BFF doivent recevoir `JWT_PUBLIC_KEY` pour les vérifier. Tant que `JWT_SECRET` est défini, les tokens HS256 restent acceptés ; sans clé privée, Core signe encore en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `PATCH /api/v1/user/me/` avec `email` : chaque vérification du mot de passe actuel consomme un budget de 5 par utilisateur sur 15 minutes, juste ou faux. Au-delà : `429` avec `Retry-After` et message générique, sans rien vérifier ni modifier. Les changements sans `email` ne sont pas limités. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- Dernier administrateur : `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` (« This user is the last active administrator: give the Admin role to another active user first. ») si aucun administrateur actif ne resterait. Rien n'est modifié. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Rôles système : `PATCH` et `PUT /api/v1/admin/roles/{id}` avec `can_be_deleted: true` sur un rôle protégé (ex. `Admin`) répondent `403` (« System roles stay protected: can_be_deleted cannot be changed. »). Un rôle personnalisé protégé reste protégé. `null` sur `PATCH` n'écrit rien. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Téléphone : `GET /api/v1/user/{id}/` expose désormais le numéro de téléphone à tout agent (il était masqué). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Répertoire : avec `DIRECTORY_GUEST_ACCESS=none` (défaut `all`), `GET /api/v1/user/` et `GET /api/v1/user/{id}/` répondent `403` aux seuls agents qui n'ont que le rôle Guest ; ils gardent leur propre fiche. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Client généré : `POST /api/v1/auth/logout` a l'`operationId` `logout_with_refresh_token` (client `logoutWithRefreshToken()`). `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections
- Sessions affichées : avant, Paramètres et Administration affichaient « Date indisponible » / « État indéterminé » et le bouton de révocation n'apparaissait pas. Maintenant les dates sont au format ISO 8601 que lit `SessionList`. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Déconnexion avec JWT expiré : avant, `POST /api/v1/sessions/revoke` et `POST /api/v1/sessions/logout` répondaient 401, le BFF effaçait les cookies et le refresh token restait valide 30 jours. Maintenant `POST /api/v1/auth/logout` révoque la session à partir du refresh token seul. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Dernier administrateur : avant, un appel pouvait retirer ou rétrograder le dernier administrateur, ou passer le rôle `Admin` en supprimable puis le supprimer, et laisser l'instance sans administrateur. Maintenant ces appels sont refusés (`409` ou `403`). (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352), [#355](https://github.com/mairie360/Core_API/pull/355))

#### Technique
- Images Docker épinglées par digest (Renovate, [#363](https://github.com/mairie360/Core_API/pull/363), [#350](https://github.com/mairie360/Core_API/pull/350)). Builder, runtime distroless et image de dev passent sur Debian 13 (`cc-debian13`, `rust:1.99-slim-trixie`), avec un test de garde ([#339](https://github.com/mairie360/Core_API/pull/339), MAIR-741).
- Workflows CI et promotion passent à CICD v5.2.2 ([#357](https://github.com/mairie360/Core_API/pull/357)), qui remplace les épinglages v4.4.x ; `ANTHROPIC_WORKSPACE_ID` est transmis à `promote-prod`.
- Mises à jour de dépendances : `mairie360_api_lib` 4.2 → 4.3 ([#346](https://github.com/mairie360/Core_API/pull/346)) ; `actix-web`, `redis` (dev) et `uuid` ([#335](https://github.com/mairie360/Core_API/pull/335), diff non fourni) ; `tokio` 1.54.1 ([#333](https://github.com/mairie360/Core_API/pull/333), diff non fourni).
- Image Database `dev-76437e3` (`anonymize_user()` et `export_user_data()`, MAIR-289) dans les compose et `.cargo/config.toml`.
- Logs de requête sans query string ni valeurs d'erreur, `mairie360_api_lib` 3.0.1, exception `RUSTSEC-2026-0387` ([#265](https://github.com/mairie360/Core_API/pull/265), MAIR-290).
- Tests ajoutés : refus de connexion indiscernables, archivage qui révoque les sessions ([#268](https://github.com/mairie360/Core_API/pull/268)), matrice d'accès ([#274](https://github.com/mairie360/Core_API/pull/274)), limites par client relayé par les BFF ([#342](https://github.com/mairie360/Core_API/pull/342)), garde des operationId ([#349](https://github.com/mairie360/Core_API/pull/349)), simulation RGPD et checklist PR ([#273](https://github.com/mairie360/Core_API/pull/273), [#272](https://github.com/mairie360/Core_API/pull/272)).
- Divers : `.gitleaksignore` (faux positifs d'exemples, [#365](https://github.com/mairie360/Core_API/pull/365), [#344](https://github.com/mairie360/Core_API/pull/344)) ; épinglages CICD v4.4.1 à v4.4.3 ([#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330)), remplacés par v5.2.2.

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : déconnexion avec le seul refresh token, sans JWT. Répond `204` (y compris pour un jeton inconnu ou déjà révoqué), `400` si le corps est invalide, `429` (budget refresh, 60/min par adresse) et `503` si la révocation n'a pas pu être publiée sur Redis (réessayer). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : clés publiques Ed25519 des JWT de session au format JWKS, sans clé privée, `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif (art. 17 RGPD) via `anonymize_user()`. `404` si le compte est inconnu, `409` si c'est le compte d'amorçage ou s'il n'existe pas d'autre administrateur actif pour reprendre groupes, projets et événements. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. Les sessions effacées sont publiées sur la liste de révocation. Avec Keycloak, le compte est désactivé puis supprimé (`502` si seule cette dernière étape échoue). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export JSON de toutes les données liées à l'utilisateur, identifiants exclus (art. 15 et 20 RGPD), `404` si le compte est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))

#### Changements de comportement
- Dates de session (`GET /api/v1/sessions/`, `GET /api/v1/sessions/history`, `GET /api/v1/admin/users/{userId}/`) : `created_at`, `expires_at` et `revoked_at` sont au format ISO 8601 UTC (`2026-09-16T08:42:11Z`) au lieu de `2026-09-16 08:42:11 UTC`. Les champs restent des chaînes ; un appelant qui parse l'ancien format doit être adapté. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- JWT de session : si `JWT_PRIVATE_KEY` est défini, Core signe en `EdDSA` (`kid`, `iss` `mairie360-core`, `aud` `mairie360`). Les autres APIs et BFF doivent recevoir `JWT_PUBLIC_KEY` pour les vérifier. Tant que `JWT_SECRET` est défini, les tokens HS256 restent acceptés ; sans clé privée, Core signe encore en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `PATCH /api/v1/user/me/` avec `email` : chaque vérification du mot de passe actuel consomme un budget de 5 par utilisateur sur 15 minutes, juste ou faux. Au-delà : `429` avec `Retry-After` et message générique, sans rien vérifier ni modifier. Les changements sans `email` ne sont pas limités. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- Dernier administrateur : `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` (« This user is the last active administrator: give the Admin role to another active user first. ») si aucun administrateur actif ne resterait. Rien n'est modifié. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Rôles système : `PATCH` et `PUT /api/v1/admin/roles/{id}` avec `can_be_deleted: true` sur un rôle protégé (ex. `Admin`) répondent `403` (« System roles stay protected: can_be_deleted cannot be changed. »). Un rôle personnalisé protégé reste protégé. `null` sur `PATCH` n'écrit rien. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Téléphone : `GET /api/v1/user/{id}/` expose désormais le numéro de téléphone à tout agent (il était masqué). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Répertoire : avec `DIRECTORY_GUEST_ACCESS=none` (défaut `all`), `GET /api/v1/user/` et `GET /api/v1/user/{id}/` répondent `403` aux seuls agents qui n'ont que le rôle Guest ; ils gardent leur propre fiche. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Client généré : `POST /api/v1/auth/logout` a l'`operationId` `logout_with_refresh_token` (client `logoutWithRefreshToken()`). `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections
- Sessions affichées : avant, Paramètres et Administration affichaient « Date indisponible » / « État indéterminé » et le bouton de révocation n'apparaissait pas. Maintenant les dates sont au format ISO 8601 que lit `SessionList`. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Déconnexion avec JWT expiré : avant, `POST /api/v1/sessions/revoke` et `POST /api/v1/sessions/logout` répondaient 401, le BFF effaçait les cookies et le refresh token restait valide 30 jours. Maintenant `POST /api/v1/auth/logout` révoque la session à partir du refresh token seul. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Dernier administrateur : avant, un appel pouvait retirer ou rétrograder le dernier administrateur, ou passer le rôle `Admin` en supprimable puis le supprimer, et laisser l'instance sans administrateur. Maintenant ces appels sont refusés (`409` ou `403`). (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352), [#355](https://github.com/mairie360/Core_API/pull/355))

#### Technique
- Images Docker épinglées par digest (Renovate, [#363](https://github.com/mairie360/Core_API/pull/363), [#350](https://github.com/mairie360/Core_API/pull/350)). Builder, runtime distroless et image de dev passent sur Debian 13 (`cc-debian13`, `rust:1.99-slim-trixie`), avec un test de garde ([#339](https://github.com/mairie360/Core_API/pull/339), MAIR-741).
- Workflows CI et promotion passent à CICD v5.2.2 ([#357](https://github.com/mairie360/Core_API/pull/357)), qui remplace les épinglages v4.4.x ; `ANTHROPIC_WORKSPACE_ID` est transmis à `promote-prod`.
- Mises à jour de dépendances : `mairie360_api_lib` 4.2 → 4.3 ([#346](https://github.com/mairie360/Core_API/pull/346)) ; `actix-web`, `redis` (dev) et `uuid` ([#335](https://github.com/mairie360/Core_API/pull/335), diff non fourni) ; `tokio` 1.54.1 ([#333](https://github.com/mairie360/Core_API/pull/333), diff non fourni).
- Image Database `dev-76437e3` (`anonymize_user()` et `export_user_data()`, MAIR-289) dans les compose et `.cargo/config.toml`.
- Logs de requête sans query string ni valeurs d'erreur, `mairie360_api_lib` 3.0.1, exception `RUSTSEC-2026-0387` ([#265](https://github.com/mairie360/Core_API/pull/265), MAIR-290).
- Tests ajoutés : refus de connexion indiscernables, archivage qui révoque les sessions ([#268](https://github.com/mairie360/Core_API/pull/268)), matrice d'accès ([#274](https://github.com/mairie360/Core_API/pull/274)), limites par client relayé par les BFF ([#342](https://github.com/mairie360/Core_API/pull/342)), garde des operationId ([#349](https://github.com/mairie360/Core_API/pull/349)), simulation RGPD et checklist PR ([#273](https://github.com/mairie360/Core_API/pull/273), [#272](https://github.com/mairie360/Core_API/pull/272)).
- Divers : `.gitleaksignore` (faux positifs d'exemples, [#365](https://github.com/mairie360/Core_API/pull/365), [#344](https://github.com/mairie360/Core_API/pull/344)) ; épinglages CICD v4.4.1 à v4.4.3 ([#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330)), remplacés par v5.2.2.

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-11-04 14:17 UTC | dev | — | `b0a53b8` | [run](https://github.com/mairie360/Core_API/actions/runs/41843590397) |
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
