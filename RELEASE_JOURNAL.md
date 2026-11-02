# Journal des releases — Core_API

<!-- release-journal-state {"dev": "3b40c2aa3067cf7d7408d05771e1cdbd5bdb9877", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-02 23:07 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `3b40c2a` | 2026-11-02 23:07 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- Nouvelle route `POST /api/v1/auth/logout` : déconnexion avec le seul refresh token, sans JWT, y compris quand le JWT a expiré. Elle révoque la session et la publie sur la liste de révocation. Elle répond toujours `204`, que le token soit connu, déjà révoqué ou inconnu, et est limitée par adresse IP (60 par minute, comme `/sessions/refresh`). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Effacement RGPD : `POST /api/v1/admin/users/{userId}/erase` (admin) appelle `anonymize_user()`, révoque les sessions du compte et supprime le compte Keycloak. La réponse contient `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions` et `keycloak_account_deleted`. `404` si le compte est inconnu, `409` pour le compte administrateur de référence ou quand aucun autre administrateur actif ne peut reprendre les objets. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Export des données : `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient toutes les données liées au compte (`export_user_data()`), sans les identifiants. `404` si le compte est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Nouvelle route publique `GET /api/v1/auth/jwks` qui publie les clés publiques de vérification des JWT de session (JWKS). (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Annuaire : la variable `DIRECTORY_GUEST_ACCESS=none` ferme `GET /api/v1/user/` et les fiches des autres utilisateurs aux agents qui n'ont que le rôle Guest (`403`). Par défaut, l'annuaire reste ouvert. Le téléphone d'un agent est désormais visible de tous les agents sur `GET /api/v1/user/{id}/`, et un compte archivé n'est visible que de lui-même et des administrateurs (sinon `404`). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `PATCH /api/v1/user/me/` avec un `email` : chaque vérification du mot de passe actuel consomme un budget de 5 par utilisateur sur 15 minutes, bonne ou mauvaise. Au-delà, la réponse est `429` avec `Retry-After`, sans vérification ni modification. Les changements sans `email` ne sont pas limités. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- Dates de session : `created_at`, `expires_at` et `revoked_at` de `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` passent du format `2026-09-16 08:42:11 UTC` au format ISO 8601 UTC `2026-09-16T08:42:11Z`. Le type reste une chaîne. Un appelant qui parse l'ancien format doit être adapté. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Rôles système : `PATCH` et `PUT /api/v1/admin/roles/{id}` répondent `403` avec `System roles stay protected: can_be_deleted cannot be changed.` quand `can_be_deleted: true` est envoyé sur un rôle système (par exemple Admin). Un rôle personnalisé qu'un administrateur protège reste ensuite protégé. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Dernier administrateur : `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` avec le message `This user is the last active administrator: give the Admin role to another active user first.` quand l'opération ne laisserait aucun administrateur actif. Rien n'est modifié. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Jetons de session : avec `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY`, Core signe les JWT en EdDSA (`iss` `mairie360-core`, `aud` `mairie360`). Sans clé privée, il signe encore en HS256. Les jetons HS256 restent acceptés tant que `JWT_SECRET` est défini. Les autres APIs et BFF doivent recevoir `JWT_PUBLIC_KEY` pour vérifier les nouveaux jetons. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Client généré : la nouvelle route de déconnexion porte l'identifiant d'opération `logout_with_refresh_token`, donc `logoutWithRefreshToken()` côté client. `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections
- Dates de session : avant, la liste des sessions affichait « Date indisponible » ou « État indéterminé », et le bouton de révocation n'apparaissait pas, car le format `… UTC` n'était pas reconnu. Maintenant, les dates sont en ISO 8601. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Déconnexion avec JWT expiré : avant, `POST /api/v1/sessions/revoke` et `POST /api/v1/sessions/logout` répondaient `401`, le BFF effaçait les cookies et le refresh token restait valide 30 jours. Maintenant, `POST /api/v1/auth/logout` révoque la session sans JWT. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Dernier administrateur : avant, `PATCH` ou `PUT` avec `can_be_deleted: true` sur le rôle Admin permettait ensuite de le renommer ou de le supprimer, sans administrateur. Maintenant, la requête est refusée en `403`. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Retrait simultané des deux derniers administrateurs : les écritures concernées se sérialisent désormais sur un verrou du rôle Admin, et la seconde voit le résultat de la première. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Journaux : la query string des requêtes (par exemple une recherche par e-mail) n'est plus journalisée. Les erreurs sont décrites par leur type et leur contexte, sans la valeur reçue (token de première connexion, e-mail). (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- CICD passe de v4.4.0 à v4.4.1, v4.4.2 puis v4.4.3 (corrections de la promotion prod), puis à v5.2.2 pour activer les contrôles RGPD de la CI et de la promotion ; un test vérifie que tous les workflows appellent la même version. ([#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330), [#357](https://github.com/mairie360/Core_API/pull/357))
- Image d'exécution distroless passée de Debian 12 à Debian 13 (CVE sur libssl3), builder et image de développement passés sur trixie ; un test vérifie que chaque image est sur Debian 13. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))
- Dépendances : mises à jour Renovate des digests `rust:1.99` (#328, #332, #334) et de tokio 1.54.1 (#333) ; `mairie360_api_lib` en 3.0.1 puis 4.3.0 ; ajout de `yaml_serde` ; exception `cargo audit` RUSTSEC-2026-0387 documentée. ([#265](https://github.com/mairie360/Core_API/pull/265), [#274](https://github.com/mairie360/Core_API/pull/274), [#346](https://github.com/mairie360/Core_API/pull/346))
- Tests et outillage : matrice d'accès vérifiée depuis l'OpenAPI, tests d'authentification RGPD et de révocation, limites de débit relayées par les BFF, simulation d'usage RGPD, checklist RGPD sur les PR, stack de marqueur RGPD dans les logs. ([#265](https://github.com/mairie360/Core_API/pull/265), [#268](https://github.com/mairie360/Core_API/pull/268), [#272](https://github.com/mairie360/Core_API/pull/272), [#273](https://github.com/mairie360/Core_API/pull/273), [#274](https://github.com/mairie360/Core_API/pull/274), [#342](https://github.com/mairie360/Core_API/pull/342))

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- Nouvelle route `POST /api/v1/auth/logout` : déconnexion avec le seul refresh token, sans JWT, y compris quand le JWT a expiré. Elle révoque la session et la publie sur la liste de révocation. Elle répond toujours `204`, que le token soit connu, déjà révoqué ou inconnu, et est limitée par adresse IP (60 par minute, comme `/sessions/refresh`). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Effacement RGPD : `POST /api/v1/admin/users/{userId}/erase` (admin) appelle `anonymize_user()`, révoque les sessions du compte et supprime le compte Keycloak. La réponse contient `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions` et `keycloak_account_deleted`. `404` si le compte est inconnu, `409` pour le compte administrateur de référence ou quand aucun autre administrateur actif ne peut reprendre les objets. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Export des données : `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient toutes les données liées au compte (`export_user_data()`), sans les identifiants. `404` si le compte est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Nouvelle route publique `GET /api/v1/auth/jwks` qui publie les clés publiques de vérification des JWT de session (JWKS). (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Annuaire : la variable `DIRECTORY_GUEST_ACCESS=none` ferme `GET /api/v1/user/` et les fiches des autres utilisateurs aux agents qui n'ont que le rôle Guest (`403`). Par défaut, l'annuaire reste ouvert. Le téléphone d'un agent est désormais visible de tous les agents sur `GET /api/v1/user/{id}/`, et un compte archivé n'est visible que de lui-même et des administrateurs (sinon `404`). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `PATCH /api/v1/user/me/` avec un `email` : chaque vérification du mot de passe actuel consomme un budget de 5 par utilisateur sur 15 minutes, bonne ou mauvaise. Au-delà, la réponse est `429` avec `Retry-After`, sans vérification ni modification. Les changements sans `email` ne sont pas limités. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- Dates de session : `created_at`, `expires_at` et `revoked_at` de `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/` passent du format `2026-09-16 08:42:11 UTC` au format ISO 8601 UTC `2026-09-16T08:42:11Z`. Le type reste une chaîne. Un appelant qui parse l'ancien format doit être adapté. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Rôles système : `PATCH` et `PUT /api/v1/admin/roles/{id}` répondent `403` avec `System roles stay protected: can_be_deleted cannot be changed.` quand `can_be_deleted: true` est envoyé sur un rôle système (par exemple Admin). Un rôle personnalisé qu'un administrateur protège reste ensuite protégé. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Dernier administrateur : `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` avec le message `This user is the last active administrator: give the Admin role to another active user first.` quand l'opération ne laisserait aucun administrateur actif. Rien n'est modifié. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Jetons de session : avec `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY`, Core signe les JWT en EdDSA (`iss` `mairie360-core`, `aud` `mairie360`). Sans clé privée, il signe encore en HS256. Les jetons HS256 restent acceptés tant que `JWT_SECRET` est défini. Les autres APIs et BFF doivent recevoir `JWT_PUBLIC_KEY` pour vérifier les nouveaux jetons. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Client généré : la nouvelle route de déconnexion porte l'identifiant d'opération `logout_with_refresh_token`, donc `logoutWithRefreshToken()` côté client. `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections
- Dates de session : avant, la liste des sessions affichait « Date indisponible » ou « État indéterminé », et le bouton de révocation n'apparaissait pas, car le format `… UTC` n'était pas reconnu. Maintenant, les dates sont en ISO 8601. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Déconnexion avec JWT expiré : avant, `POST /api/v1/sessions/revoke` et `POST /api/v1/sessions/logout` répondaient `401`, le BFF effaçait les cookies et le refresh token restait valide 30 jours. Maintenant, `POST /api/v1/auth/logout` révoque la session sans JWT. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Dernier administrateur : avant, `PATCH` ou `PUT` avec `can_be_deleted: true` sur le rôle Admin permettait ensuite de le renommer ou de le supprimer, sans administrateur. Maintenant, la requête est refusée en `403`. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Retrait simultané des deux derniers administrateurs : les écritures concernées se sérialisent désormais sur un verrou du rôle Admin, et la seconde voit le résultat de la première. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- Journaux : la query string des requêtes (par exemple une recherche par e-mail) n'est plus journalisée. Les erreurs sont décrites par leur type et leur contexte, sans la valeur reçue (token de première connexion, e-mail). (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- CICD passe de v4.4.0 à v4.4.1, v4.4.2 puis v4.4.3 (corrections de la promotion prod), puis à v5.2.2 pour activer les contrôles RGPD de la CI et de la promotion ; un test vérifie que tous les workflows appellent la même version. ([#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330), [#357](https://github.com/mairie360/Core_API/pull/357))
- Image d'exécution distroless passée de Debian 12 à Debian 13 (CVE sur libssl3), builder et image de développement passés sur trixie ; un test vérifie que chaque image est sur Debian 13. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))
- Dépendances : mises à jour Renovate des digests `rust:1.99` (#328, #332, #334) et de tokio 1.54.1 (#333) ; `mairie360_api_lib` en 3.0.1 puis 4.3.0 ; ajout de `yaml_serde` ; exception `cargo audit` RUSTSEC-2026-0387 documentée. ([#265](https://github.com/mairie360/Core_API/pull/265), [#274](https://github.com/mairie360/Core_API/pull/274), [#346](https://github.com/mairie360/Core_API/pull/346))
- Tests et outillage : matrice d'accès vérifiée depuis l'OpenAPI, tests d'authentification RGPD et de révocation, limites de débit relayées par les BFF, simulation d'usage RGPD, checklist RGPD sur les PR, stack de marqueur RGPD dans les logs. ([#265](https://github.com/mairie360/Core_API/pull/265), [#268](https://github.com/mairie360/Core_API/pull/268), [#272](https://github.com/mairie360/Core_API/pull/272), [#273](https://github.com/mairie360/Core_API/pull/273), [#274](https://github.com/mairie360/Core_API/pull/274), [#342](https://github.com/mairie360/Core_API/pull/342))

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
