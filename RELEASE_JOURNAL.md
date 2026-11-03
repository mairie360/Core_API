# Journal des releases — Core_API

<!-- release-journal-state {"dev": "ca6521a4eeec03f605348c9496d3bdbdd8f34958", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-03 21:33 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `ca6521a` | 2026-11-03 21:33 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés

- `POST /api/v1/admin/users/{userId}/erase` (admin) efface un compte pour de bon via `anonymize_user()` de la base : identité effacée, appartenances, sessions et préférences supprimées, événements privés supprimés, groupes, projets et événements transmis à un administrateur actif. Les sessions révoquées sont publiées sur la liste de révocation, et le compte Keycloak est désactivé puis supprimé. Réponse : `{ user_id, already_anonymized, handed_over_to, deleted_private_events, revoked_sessions, keycloak_account_deleted }`. Un `P0002` répond `404`, un `23001` répond `409`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient `export_user_data()` de la base, sans les identifiants. Un utilisateur inconnu répond `404`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `POST /api/v1/auth/logout` (public) révoque la session liée à un refresh token, sans JWT, et la publie sur la liste de révocation. Il répond toujours `204` (token inconnu, déjà révoqué ou valide), limité à 60 par minute et par adresse. Il répond `503` si la publication Redis échoue. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) publie les clés publiques Ed25519 des JWT de session, avec `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Les JWT de session peuvent être émis en EdDSA, avec `iss` `mairie360-core` et `aud` `mairie360`, lorsque `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY` sont définis. Sans clé privée, Core signe toujours en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `PATCH /api/v1/user/me/` vérifie le mot de passe actuel au plus 5 fois par utilisateur et par 15 minutes, bonne ou mauvaise réponse. Au-delà, il répond `429` avec `Retry-After`. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))

#### Changements de comportement

- Les dates des sessions (`created_at`, `expires_at`, `revoked_at`) sont au format ISO 8601 UTC, par exemple `2026-09-16T08:42:11Z`, au lieu de `2026-09-16 08:42:11 UTC`, sur `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/`. Un appelant qui parse l'ancien format doit être adapté. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` si l'opération laisserait aucun administrateur actif, et rien n'est modifié. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- `PATCH` et `PUT /api/v1/admin/roles/{id}` répondent `403` avec `System roles stay protected: can_be_deleted cannot be changed.` lorsque `can_be_deleted: true` est envoyé pour un rôle système. Un rôle personnalisé protégé le reste. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- `GET /api/v1/user/` et `GET /api/v1/user/{id}/` répondent `403` aux agents qui n'ont que le rôle Guest lorsque `DIRECTORY_GUEST_ACCESS=none` ; ils lisent toujours leur propre fiche. La valeur par défaut `all` ne change rien. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Le téléphone d'un agent est visible par tous les agents sur `GET /api/v1/user/{id}/`. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Le formulaire de login renvoie la même réponse (statut, en-têtes et corps) pour un e-mail inconnu, un mauvais mot de passe, une première connexion avec mauvais mot de passe et un compte archivé. (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268))
- `POST /api/v1/user/me/` refuse un changement d'e-mail répété au-delà du quota de vérifications de mot de passe (`429`, voir ci-dessus). (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- L'opération `POST /api/v1/auth/logout` a pour `operationId` `logout_with_refresh_token`, donc le client généré l'appelle `logoutWithRefreshToken()`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections

- Un administrateur pouvait retirer la protection du rôle `Admin` par `PATCH` avec `can_be_deleted: true`, puis le renommer ou le supprimer. Maintenant la requête est refusée en `403`. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Un administrateur pouvait rétrograder, retirer ou archiver le dernier administrateur actif. Maintenant ces opérations répondent `409`. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- La déconnexion échouait avec un JWT expiré, et le refresh token restait valide. Maintenant `POST /api/v1/auth/logout` révoque la session sans JWT. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Les dates de session n'étaient pas lisibles par `SessionList` (« Date indisponible »), et le bouton de révocation n'apparaissait pas. Maintenant elles sont en ISO 8601. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Le changement d'e-mail permettait de deviner le mot de passe sans limite. Maintenant il est limité à 5 vérifications par utilisateur et par 15 minutes. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- Un administrateur archivé perdait l'accès, mais ses sessions restaient ouvertes. Maintenant l'archivage révoque les sessions, et le compte ne peut plus se connecter. (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268))
- Les journaux de requêtes contenaient la query string (e-mail recherché) et le `Debug` des erreurs (token de première connexion). Ils n'affichent plus ces valeurs. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Les images d'exécution passent en Debian 13 (distroless `cc-debian13`, `rust:1.99-slim-trixie`), ce qui corrige la CVE-2026-84782 de `libssl3`. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))

#### Technique

- Simulation d'usage RGPD (`gdpr-simulation.yaml`, `gdpr_simulation_test.sh`, `docker-compose-gdpr-simulation.yml`), matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI, checklist RGPD des PR (`gdpr-pr.yml`), marqueur RGPD des journaux (`gdpr-marker.yaml`) : ajout de tests et de workflows, sans effet direct pour les utilisateurs. (MAIR-497, MAIR-288, MAIR-295, MAIR-290)
- Mises à jour de dépendances et d'images : `mairie360_api_lib` 4.2.0 → 4.3.0, tokio 1.54.1, digests Docker (rust, redis, postgres, nginx, node, curl, k6, wiremock, zap, newman) épinglés, CICD v4.4.3 → v5.2.2 dans les workflows. (MAIR-771, MAIR-612)
- Tests et contrôles CI sans effet visible : `tests/operation_ids_test.rs`, `tests/dockerfile_test.rs`, `tests/mair_771_cicd_version.rs`, tests de limitation de débit BFF (MAIR-679).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés

- `POST /api/v1/admin/users/{userId}/erase` (admin) efface un compte pour de bon via `anonymize_user()` de la base : identité effacée, appartenances, sessions et préférences supprimées, événements privés supprimés, groupes, projets et événements transmis à un administrateur actif. Les sessions révoquées sont publiées sur la liste de révocation, et le compte Keycloak est désactivé puis supprimé. Réponse : `{ user_id, already_anonymized, handed_over_to, deleted_private_events, revoked_sessions, keycloak_account_deleted }`. Un `P0002` répond `404`, un `23001` répond `409`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient `export_user_data()` de la base, sans les identifiants. Un utilisateur inconnu répond `404`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `POST /api/v1/auth/logout` (public) révoque la session liée à un refresh token, sans JWT, et la publie sur la liste de révocation. Il répond toujours `204` (token inconnu, déjà révoqué ou valide), limité à 60 par minute et par adresse. Il répond `503` si la publication Redis échoue. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) publie les clés publiques Ed25519 des JWT de session, avec `Cache-Control: public, max-age=300`. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Les JWT de session peuvent être émis en EdDSA, avec `iss` `mairie360-core` et `aud` `mairie360`, lorsque `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY` sont définis. Sans clé privée, Core signe toujours en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `PATCH /api/v1/user/me/` vérifie le mot de passe actuel au plus 5 fois par utilisateur et par 15 minutes, bonne ou mauvaise réponse. Au-delà, il répond `429` avec `Retry-After`. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))

#### Changements de comportement

- Les dates des sessions (`created_at`, `expires_at`, `revoked_at`) sont au format ISO 8601 UTC, par exemple `2026-09-16T08:42:11Z`, au lieu de `2026-09-16 08:42:11 UTC`, sur `GET /api/v1/sessions/`, `GET /api/v1/sessions/history` et `GET /api/v1/admin/users/{userId}/`. Un appelant qui parse l'ancien format doit être adapté. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` répondent `409` si l'opération laisserait aucun administrateur actif, et rien n'est modifié. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- `PATCH` et `PUT /api/v1/admin/roles/{id}` répondent `403` avec `System roles stay protected: can_be_deleted cannot be changed.` lorsque `can_be_deleted: true` est envoyé pour un rôle système. Un rôle personnalisé protégé le reste. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- `GET /api/v1/user/` et `GET /api/v1/user/{id}/` répondent `403` aux agents qui n'ont que le rôle Guest lorsque `DIRECTORY_GUEST_ACCESS=none` ; ils lisent toujours leur propre fiche. La valeur par défaut `all` ne change rien. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Le téléphone d'un agent est visible par tous les agents sur `GET /api/v1/user/{id}/`. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Le formulaire de login renvoie la même réponse (statut, en-têtes et corps) pour un e-mail inconnu, un mauvais mot de passe, une première connexion avec mauvais mot de passe et un compte archivé. (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268))
- `POST /api/v1/user/me/` refuse un changement d'e-mail répété au-delà du quota de vérifications de mot de passe (`429`, voir ci-dessus). (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- L'opération `POST /api/v1/auth/logout` a pour `operationId` `logout_with_refresh_token`, donc le client généré l'appelle `logoutWithRefreshToken()`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))

#### Corrections

- Un administrateur pouvait retirer la protection du rôle `Admin` par `PATCH` avec `can_be_deleted: true`, puis le renommer ou le supprimer. Maintenant la requête est refusée en `403`. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Un administrateur pouvait rétrograder, retirer ou archiver le dernier administrateur actif. Maintenant ces opérations répondent `409`. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))
- La déconnexion échouait avec un JWT expiré, et le refresh token restait valide. Maintenant `POST /api/v1/auth/logout` révoque la session sans JWT. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Les dates de session n'étaient pas lisibles par `SessionList` (« Date indisponible »), et le bouton de révocation n'apparaissait pas. Maintenant elles sont en ISO 8601. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Le changement d'e-mail permettait de deviner le mot de passe sans limite. Maintenant il est limité à 5 vérifications par utilisateur et par 15 minutes. (MAIR-746, [#359](https://github.com/mairie360/Core_API/pull/359))
- Un administrateur archivé perdait l'accès, mais ses sessions restaient ouvertes. Maintenant l'archivage révoque les sessions, et le compte ne peut plus se connecter. (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268))
- Les journaux de requêtes contenaient la query string (e-mail recherché) et le `Debug` des erreurs (token de première connexion). Ils n'affichent plus ces valeurs. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Les images d'exécution passent en Debian 13 (distroless `cc-debian13`, `rust:1.99-slim-trixie`), ce qui corrige la CVE-2026-84782 de `libssl3`. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))

#### Technique

- Simulation d'usage RGPD (`gdpr-simulation.yaml`, `gdpr_simulation_test.sh`, `docker-compose-gdpr-simulation.yml`), matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI, checklist RGPD des PR (`gdpr-pr.yml`), marqueur RGPD des journaux (`gdpr-marker.yaml`) : ajout de tests et de workflows, sans effet direct pour les utilisateurs. (MAIR-497, MAIR-288, MAIR-295, MAIR-290)
- Mises à jour de dépendances et d'images : `mairie360_api_lib` 4.2.0 → 4.3.0, tokio 1.54.1, digests Docker (rust, redis, postgres, nginx, node, curl, k6, wiremock, zap, newman) épinglés, CICD v4.4.3 → v5.2.2 dans les workflows. (MAIR-771, MAIR-612)
- Tests et contrôles CI sans effet visible : `tests/operation_ids_test.rs`, `tests/dockerfile_test.rs`, `tests/mair_771_cicd_version.rs`, tests de limitation de débit BFF (MAIR-679).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
