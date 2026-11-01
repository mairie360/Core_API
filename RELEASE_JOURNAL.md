# Journal des releases — Core_API

<!-- release-journal-state {"dev": "72dca5b230cd81de506fab575698792fc316140e", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-01 22:33 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `72dca5b` | 2026-11-01 22:33 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : corps `{ refresh_token }`. Révoque la session de ce refresh token, sans JWT, et répond toujours `204`, que le token soit connu, déjà révoqué ou inconnu. `400` si le corps est invalide, `429` au-delà de 60 par minute et par adresse, `503` si la révocation n'a pas pu être publiée dans Redis (il suffit de rééssayer). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : publie les clés publiques des JWT de session au format JWKS, sans la clé privée, avec `Cache-Control: public, max-age=300`. Le tableau `keys` est vide tant que l'instance signe en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif d'un compte (art. 17 RGPD) via `anonymize_user()` de la base. Les sessions sont publiées dans la liste de révocation. Côté Keycloak, le compte est désactivé puis supprimé ; `404` si l'utilisateur est inconnu, `409` pour l'administrateur initial ou s'il n'existe aucun autre administrateur actif pour reprendre les objets. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` (admin) : renvoient `export_user_data()` tel quel, soit toutes les données liées à l'utilisateur sans les identifiants. `404` si l'utilisateur est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` (valeur par défaut `all`) : les agents qui n'ont que le rôle Guest reçoivent `403` sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur, et gardent leur propre fiche. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tous les agents. Un compte archivé n'est visible que de lui-même et des administrateurs (`404` pour les autres). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Les JWT de session sont signés en EdDSA quand `JWT_PRIVATE_KEY` est défini, avec `kid`, `iss` `mairie360-core` et `aud` `mairie360`. Sans cette clé, Core signe toujours en HS256, et les HS256 restent acceptés tant que `JWT_SECRET` est défini. Les autres API et BFF doivent recevoir `JWT_PUBLIC_KEY` pour vérifier les nouveaux tokens. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/auth/logout` et les nouvelles routes d'effacement et d'export ajoutent des entrées à la matrice d'accès (`access-matrix.yaml`). La liste des opérations du contrat change donc pour les appelants qui la lisent.

#### Corrections
- Déconnexion avec un JWT expiré : avant, `POST /api/v1/sessions/logout` et `POST /api/v1/sessions/revoke` répondaient `401`, donc le refresh token restait valide 30 jours côté serveur. Maintenant, `POST /api/v1/auth/logout` révoque la session à partir du seul refresh token. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Nom client des routes de déconnexion : avant, le nouveau handler portait le même nom que `POST /api/v1/sessions/logout`, et le client généré renommait l'existant en `logout2()`. Maintenant, la nouvelle route s'appelle `logoutWithRefreshToken()` et `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#348](https://github.com/mairie360/Core_API/pull/348) via [#349](https://github.com/mairie360/Core_API/pull/349))
- Journaux de requêtes : avant, la chaîne de requête (par exemple un e-mail recherché) et le `Debug` des erreurs (jeton de première connexion) étaient journalisés. Maintenant, la chaîne de requête est retirée du span racine et les erreurs sont décrites sans leurs valeurs. Les erreurs de base sont décrites par SQLSTATE, contrainte, table et colonne. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- Tests de non-régression : routes d'effacement et de la matrice d'accès, rate limit derrière le BFF, refus de connexion indistinguables, blocage d'un compte archivé, et contrôle de la CI sur les noms d'opérations ([#342](https://github.com/mairie360/Core_API/pull/342), [#268](https://github.com/mairie360/Core_API/pull/268), [#274](https://github.com/mairie360/Core_API/pull/274), [#349](https://github.com/mairie360/Core_API/pull/349)).
- Images Docker : runtime `distroless/cc-debian13` et builder `rust:1.99-slim-trixie`, avec un test qui vérifie que toutes les images sont en Debian 13 ([#339](https://github.com/mairie360/Core_API/pull/339)).
- Base de données : `TEST_DB_VERSION` et les compose passent à `dev-76437e3`, qui contient `anonymize_user()` et `export_user_data()` ([#277](https://github.com/mairie360/Core_API/pull/277)).
- Simulation RGPD, workflow de checklist RGPD des PR et publication des clés de test ([#273](https://github.com/mairie360/Core_API/pull/273), [#272](https://github.com/mairie360/Core_API/pull/272), [#346](https://github.com/mairie360/Core_API/pull/346)).
- Dépendances et CI : Renovate (digest `rust:1.99-bookworm`, tokio 1.54.1) et mise à jour de `mairie360/CICD` vers v4.4.3 ([#333](https://github.com/mairie360/Core_API/pull/333), [#334](https://github.com/mairie360/Core_API/pull/334), [#330](https://github.com/mairie360/Core_API/pull/330)).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : corps `{ refresh_token }`. Révoque la session de ce refresh token, sans JWT, et répond toujours `204`, que le token soit connu, déjà révoqué ou inconnu. `400` si le corps est invalide, `429` au-delà de 60 par minute et par adresse, `503` si la révocation n'a pas pu être publiée dans Redis (il suffit de rééssayer). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : publie les clés publiques des JWT de session au format JWKS, sans la clé privée, avec `Cache-Control: public, max-age=300`. Le tableau `keys` est vide tant que l'instance signe en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif d'un compte (art. 17 RGPD) via `anonymize_user()` de la base. Les sessions sont publiées dans la liste de révocation. Côté Keycloak, le compte est désactivé puis supprimé ; `404` si l'utilisateur est inconnu, `409` pour l'administrateur initial ou s'il n'existe aucun autre administrateur actif pour reprendre les objets. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` (admin) : renvoient `export_user_data()` tel quel, soit toutes les données liées à l'utilisateur sans les identifiants. `404` si l'utilisateur est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` (valeur par défaut `all`) : les agents qui n'ont que le rôle Guest reçoivent `403` sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur, et gardent leur propre fiche. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tous les agents. Un compte archivé n'est visible que de lui-même et des administrateurs (`404` pour les autres). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Les JWT de session sont signés en EdDSA quand `JWT_PRIVATE_KEY` est défini, avec `kid`, `iss` `mairie360-core` et `aud` `mairie360`. Sans cette clé, Core signe toujours en HS256, et les HS256 restent acceptés tant que `JWT_SECRET` est défini. Les autres API et BFF doivent recevoir `JWT_PUBLIC_KEY` pour vérifier les nouveaux tokens. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/auth/logout` et les nouvelles routes d'effacement et d'export ajoutent des entrées à la matrice d'accès (`access-matrix.yaml`). La liste des opérations du contrat change donc pour les appelants qui la lisent.

#### Corrections
- Déconnexion avec un JWT expiré : avant, `POST /api/v1/sessions/logout` et `POST /api/v1/sessions/revoke` répondaient `401`, donc le refresh token restait valide 30 jours côté serveur. Maintenant, `POST /api/v1/auth/logout` révoque la session à partir du seul refresh token. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Nom client des routes de déconnexion : avant, le nouveau handler portait le même nom que `POST /api/v1/sessions/logout`, et le client généré renommait l'existant en `logout2()`. Maintenant, la nouvelle route s'appelle `logoutWithRefreshToken()` et `logout()` reste sur `POST /api/v1/sessions/logout`. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- Journaux de requêtes : avant, la chaîne de requête (par exemple un e-mail recherché) et le `Debug` des erreurs (jeton de première connexion) étaient journalisés. Maintenant, la chaîne de requête est retirée du span racine et les erreurs sont décrites sans leurs valeurs. Les erreurs de base sont décrites par SQLSTATE, contrainte, table et colonne. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- Tests de non-régression : routes d'effacement et de la matrice d'accès, rate limit derrière le BFF, refus de connexion indistinguables, blocage d'un compte archivé, et contrôle de la CI sur les noms d'opérations ([#342](https://github.com/mairie360/Core_API/pull/342), [#268](https://github.com/mairie360/Core_API/pull/268), [#274](https://github.com/mairie360/Core_API/pull/274), [#349](https://github.com/mairie360/Core_API/pull/349)).
- Images Docker : runtime `distroless/cc-debian13` et builder `rust:1.99-slim-trixie`, avec un test qui vérifie que toutes les images sont en Debian 13 ([#339](https://github.com/mairie360/Core_API/pull/339)).
- Base de données : `TEST_DB_VERSION` et les compose passent à `dev-76437e3`, qui contient `anonymize_user()` et `export_user_data()` ([#277](https://github.com/mairie360/Core_API/pull/277)).
- Simulation RGPD, workflow de checklist RGPD des PR et publication des clés de test ([#273](https://github.com/mairie360/Core_API/pull/273), [#272](https://github.com/mairie360/Core_API/pull/272), [#346](https://github.com/mairie360/Core_API/pull/346)).
- Dépendances et CI : Renovate (digest `rust:1.99-bookworm`, tokio 1.54.1) et mise à jour de `mairie360/CICD` vers v4.4.3 ([#333](https://github.com/mairie360/Core_API/pull/333), [#334](https://github.com/mairie360/Core_API/pull/334), [#330](https://github.com/mairie360/Core_API/pull/330)).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
