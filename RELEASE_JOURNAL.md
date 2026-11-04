# Journal des releases — Core_API

<!-- release-journal-state {"dev": "690f32b51732e10d965811e4538ee90e86aaba33", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-04 14:39 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `690f32b` | 2026-11-04 14:39 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- Accès temporaire à la console Keycloak : `POST /api/v1/admin/keycloak/console-access` (Admin, 10 par administrateur et par 15 minutes) répond `{ url, expires_in: 120 }` avec un lien à usage unique valable 2 minutes. `GET /admin-access?token=` pose le cookie `__Secure-mairie360-kc-console` (30 minutes) et redirige en `303` vers la console du realm. `GET /internal/keycloak-console/verify` (ForwardAuth Traefik) répond `200` ou `401`. Les trois routes répondent `503` sans `KEYCLOAK_ISSUER` (ou `KEYCLOAK_REALM_URL`) et sans les clés de session. (MAIR-609, [#364](https://github.com/mairie360/Core_API/pull/364))
- `GET /api/v1/auth/jwks` publie les clés publiques des JWT de session au format JWKS, sans clé privée. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/auth/logout` : déconnexion avec le seul refresh token, sans JWT. Répond toujours `204`, révoque la session et la publie sur la liste de révocation Redis. Budget de 60 par minute et par adresse. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `POST /api/v1/admin/users/{userId}/erase` : effacement définitif (anonymisation via `anonymize_user()`, sessions révoquées, compte Keycloak supprimé). Répond `409` pour le compte administrateur d'origine ou s'il n'existe aucun autre administrateur actif pour reprendre groupes, projets et événements. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient les données du compte (`export_user_data()`), identifiants exclus, et `404` pour un compte inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `DIRECTORY_GUEST_ACCESS=none` ferme l'annuaire aux agents qui n'ont que le rôle Guest : `GET /api/v1/user/` et la fiche des autres utilisateurs répondent `403`, leur propre fiche reste accessible. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- Les JWT de session sont signés en EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`) dès que `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY` sont définis. Les HS256 restent acceptés tant que `JWT_SECRET` est défini. Les autres services doivent recevoir `JWT_PUBLIC_KEY`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- L'opération OpenAPI de `POST /api/v1/auth/logout` s'appelle `logout_with_refresh_token` : le client généré l'expose sous `logoutWithRefreshToken()`, et `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- `POST /api/v1/admin/users/{userId}/roles/`, `DELETE /api/v1/admin/users/{userId}/roles/{roleId}` et `DELETE /api/v1/admin/users/{userId}/` répondent `409` (message « This user is the last active administrator ») si l'opération ne laisse aucun administrateur actif. Rien n'est modifié. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- `PATCH` et `PUT /api/v1/admin/roles/{id}` avec `can_be_deleted: true` sur un rôle système (`can_be_deleted = false`, par exemple `Admin`) répondent `403` : « System roles stay protected: can_be_deleted cannot be changed. ». Sur un tel rôle, `PATCH` n'accepte plus que `false` ou une absence du champ. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))

#### Corrections
- Dates de session : `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` renvoyaient `2026-09-16 08:42:11 UTC`, que `SessionList` ne lit pas (« Date indisponible », bouton de révocation absent). Maintenant `2026-09-16T08:42:11Z` (ISO 8601 UTC). Les champs restent des chaînes. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Changement d'e-mail par `PATCH /api/v1/user/me/` : avant, le mot de passe actuel pouvait être essayé sans limite. Maintenant, 5 vérifications par utilisateur et par 15 minutes, justes ou fausses ; au-delà, `429` avec `Retry-After`, sans rien vérifier ni modifier. Les changements sans e-mail ne sont pas limités. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- Dernier administrateur : avant, un administrateur pouvait en rétrograder, retirer ou archiver le dernier actif, puis renommer ou supprimer le rôle `Admin` après l'avoir déprotégé. Maintenant ces opérations répondent `409` ou `403`. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352) et [#355](https://github.com/mairie360/Core_API/pull/355))
- Déconnexion avec un JWT expiré : avant, `POST /api/v1/sessions/logout` répondait `401` et le refresh token restait valable 30 jours. Maintenant `POST /api/v1/auth/logout` révoque la session sans JWT. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))

#### Technique
- Dépendances et images : `mairie360_api_lib` passe en 4.3.0, mises à jour actix-web, redis, uuid et tokio, digests Docker épinglés (Renovate), image d'exécution en distroless Debian 13 (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)), base de test `dev-76437e3`.
- CI et tests : pins CICD passés de v4.4.0 à v5.2.2 avec contrôle de version (MAIR-771, [#357](https://github.com/mairie360/Core_API/pull/357)), checklist RGPD des PR et simulation d'usage RGPD ([#272](https://github.com/mairie360/Core_API/pull/272), [#273](https://github.com/mairie360/Core_API/pull/273)), matrice d'accès vérifiée depuis l'OpenAPI ([#274](https://github.com/mairie360/Core_API/pull/274)), tests de limitation par client relayé par les BFF ([#342](https://github.com/mairie360/Core_API/pull/342)) et journaux sans valeurs personnelles ([#265](https://github.com/mairie360/Core_API/pull/265)).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- Accès temporaire à la console Keycloak : `POST /api/v1/admin/keycloak/console-access` (Admin, 10 par administrateur et par 15 minutes) répond `{ url, expires_in: 120 }` avec un lien à usage unique valable 2 minutes. `GET /admin-access?token=` pose le cookie `__Secure-mairie360-kc-console` (30 minutes) et redirige en `303` vers la console du realm. `GET /internal/keycloak-console/verify` (ForwardAuth Traefik) répond `200` ou `401`. Les trois routes répondent `503` sans `KEYCLOAK_ISSUER` (ou `KEYCLOAK_REALM_URL`) et sans les clés de session. (MAIR-609, [#364](https://github.com/mairie360/Core_API/pull/364))
- `GET /api/v1/auth/jwks` publie les clés publiques des JWT de session au format JWKS, sans clé privée. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/auth/logout` : déconnexion avec le seul refresh token, sans JWT. Répond toujours `204`, révoque la session et la publie sur la liste de révocation Redis. Budget de 60 par minute et par adresse. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `POST /api/v1/admin/users/{userId}/erase` : effacement définitif (anonymisation via `anonymize_user()`, sessions révoquées, compte Keycloak supprimé). Répond `409` pour le compte administrateur d'origine ou s'il n'existe aucun autre administrateur actif pour reprendre groupes, projets et événements. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient les données du compte (`export_user_data()`), identifiants exclus, et `404` pour un compte inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `DIRECTORY_GUEST_ACCESS=none` ferme l'annuaire aux agents qui n'ont que le rôle Guest : `GET /api/v1/user/` et la fiche des autres utilisateurs répondent `403`, leur propre fiche reste accessible. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- Les JWT de session sont signés en EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`) dès que `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY` sont définis. Les HS256 restent acceptés tant que `JWT_SECRET` est défini. Les autres services doivent recevoir `JWT_PUBLIC_KEY`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- L'opération OpenAPI de `POST /api/v1/auth/logout` s'appelle `logout_with_refresh_token` : le client généré l'expose sous `logoutWithRefreshToken()`, et `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- `POST /api/v1/admin/users/{userId}/roles/`, `DELETE /api/v1/admin/users/{userId}/roles/{roleId}` et `DELETE /api/v1/admin/users/{userId}/` répondent `409` (message « This user is the last active administrator ») si l'opération ne laisse aucun administrateur actif. Rien n'est modifié. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- `PATCH` et `PUT /api/v1/admin/roles/{id}` avec `can_be_deleted: true` sur un rôle système (`can_be_deleted = false`, par exemple `Admin`) répondent `403` : « System roles stay protected: can_be_deleted cannot be changed. ». Sur un tel rôle, `PATCH` n'accepte plus que `false` ou une absence du champ. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))

#### Corrections
- Dates de session : `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` renvoyaient `2026-09-16 08:42:11 UTC`, que `SessionList` ne lit pas (« Date indisponible », bouton de révocation absent). Maintenant `2026-09-16T08:42:11Z` (ISO 8601 UTC). Les champs restent des chaînes. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Changement d'e-mail par `PATCH /api/v1/user/me/` : avant, le mot de passe actuel pouvait être essayé sans limite. Maintenant, 5 vérifications par utilisateur et par 15 minutes, justes ou fausses ; au-delà, `429` avec `Retry-After`, sans rien vérifier ni modifier. Les changements sans e-mail ne sont pas limités. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- Dernier administrateur : avant, un administrateur pouvait en rétrograder, retirer ou archiver le dernier actif, puis renommer ou supprimer le rôle `Admin` après l'avoir déprotégé. Maintenant ces opérations répondent `409` ou `403`. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352) et [#355](https://github.com/mairie360/Core_API/pull/355))
- Déconnexion avec un JWT expiré : avant, `POST /api/v1/sessions/logout` répondait `401` et le refresh token restait valable 30 jours. Maintenant `POST /api/v1/auth/logout` révoque la session sans JWT. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))

#### Technique
- Dépendances et images : `mairie360_api_lib` passe en 4.3.0, mises à jour actix-web, redis, uuid et tokio, digests Docker épinglés (Renovate), image d'exécution en distroless Debian 13 (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)), base de test `dev-76437e3`.
- CI et tests : pins CICD passés de v4.4.0 à v5.2.2 avec contrôle de version (MAIR-771, [#357](https://github.com/mairie360/Core_API/pull/357)), checklist RGPD des PR et simulation d'usage RGPD ([#272](https://github.com/mairie360/Core_API/pull/272), [#273](https://github.com/mairie360/Core_API/pull/273)), matrice d'accès vérifiée depuis l'OpenAPI ([#274](https://github.com/mairie360/Core_API/pull/274)), tests de limitation par client relayé par les BFF ([#342](https://github.com/mairie360/Core_API/pull/342)) et journaux sans valeurs personnelles ([#265](https://github.com/mairie360/Core_API/pull/265)).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-11-04 14:39 UTC | dev | — | `690f32b` | [run](https://github.com/mairie360/Core_API/actions/runs/41847588630) |
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
