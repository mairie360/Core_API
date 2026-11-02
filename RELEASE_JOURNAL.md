# Journal des releases — Core_API

<!-- release-journal-state {"dev": "1843364ab74100fca9f48b402a482bab1fea568a", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-02 17:51 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `1843364` | 2026-11-02 17:51 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/auth/logout` (nouveau, public) : ferme la session désignée par son refresh token, sans JWT, y compris quand le JWT d'accès a expiré. Répond toujours `204` (token inconnu, déjà révoqué ou valide), `400` si le corps est mal formé, `429` au-delà de 60 appels par minute et par adresse, et `503` si la révocation n'a pas pu être publiée sur Redis (à réessayer). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (nouveau, public) : publie les clés publiques des JWT de session au format JWKS, jamais la clé privée. Le tableau `keys` est vide tant que l'instance signe en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif d'un compte via `anonymize_user()`, avec révocation de ses sessions. Répond `404` pour un utilisateur inconnu et `409` si le compte est celui de l'administrateur initial ou s'il n'existe aucun autre administrateur actif pour reprendre groupes, projets et événements. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. Avec un client Keycloak confidentiel, le compte Keycloak est désactivé avant l'effacement, réactivé si Core refuse, puis supprimé ; `502` si seule cette dernière étape échoue. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export des données de l'utilisateur tel que renvoyé par `export_user_data()`, `404` pour un utilisateur inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` : ferme l'annuaire aux agents qui n'ont que le rôle Guest. `GET /api/v1/user/` et `GET /api/v1/user/{id}/` d'un autre utilisateur répondent alors `403`, leur propre fiche reste lisible. Valeur par défaut `all`, donc sans effet tant qu'elle n'est pas posée. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tout agent (il était masqué). Un compte archivé ne répond plus qu'à lui-même et aux administrateurs, `404` pour les autres. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- `GET /api/v1/roles/` et les routes de l'annuaire restent ouvertes aux cinq rôles ; seul le réglage `DIRECTORY_GUEST_ACCESS=none` ferme l'annuaire aux agents Guest. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Corrections
- Dates des sessions (`GET /api/v1/sessions/`, `GET /api/v1/sessions/history`, `GET /api/v1/admin/users/{userId}/`) : avant, `created_at`, `expires_at` et `revoked_at` étaient au format `2026-09-16 08:42:11 UTC`, que `SessionList` ne sait pas lire : les paramètres et l'administration affichaient « Date indisponible » ou « État indéterminé » et le bouton de révocation n'apparaissait pas. Maintenant, ce sont des dates ISO 8601 UTC à la seconde (`2026-09-16T08:42:11Z`). Le type reste `string`. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Rôle Admin : avant, `PATCH` ou `PUT /api/v1/admin/roles/{id}` avec `can_be_deleted: true` retirait la protection du rôle système `Admin`, qui pouvait ensuite être renommé ou supprimé, laissant l'instance sans administrateur. Maintenant, la requête répond `403` (`System roles stay protected: can_be_deleted cannot be changed.`) et un rôle personnalisé protégé le reste. Sur PATCH, `null` est traité comme un champ absent. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Dernier administrateur actif : avant, `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` pouvaient retirer le dernier administrateur. Maintenant, ces écritures répondent `409` (`This user is the last active administrator: give the Admin role to another active user first.`) et ne modifient rien. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))

#### Technique
- Noms d'opération : le nouveau handler de `POST /api/v1/auth/logout` porte `operation_id = "logout_with_refresh_token"`, pour que le client généré garde `logout()` sur `POST /api/v1/sessions/logout` (le nouveau est `logoutWithRefreshToken()`). Test `operation_ids_test.rs` ajouté. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- Clés de session : `session_jwt.rs` délègue à `mairie360_api_lib` (4.3.0) ; variables `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY` dans les compose, `stack_secrets.sh` génère une paire par run. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Logs : les spans de requête n'emportent plus la query string ni le `Debug` des erreurs (mise à jour de `mairie360_api_lib` en 3.0.1 et tests du marqueur RGPD). (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Matrice d'accès `access-matrix.yaml` et test qui vérifie chaque opération de l'OpenAPI (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274)) ; tests d'authentification RGPD sur les refus de connexion et la révocation après archivage (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268)) ; tests de rate limit derrière BFF (MAIR-679, [#342](https://github.com/mairie360/Core_API/pull/342)). Aucun changement de code métier pour ces trois.
- Image d'exécution des APIs passée en distroless Debian 13 et builder en trixie, avec test `dockerfile_test.rs` (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)).
- Simulation d'usage RGPD, checklist RGPD des PR et règle `ignore` de cargo audit (MAIR-497, MAIR-295, [#273](https://github.com/mairie360/Core_API/pull/273), [#272](https://github.com/mairie360/Core_API/pull/272), [#265](https://github.com/mairie360/Core_API/pull/265)).
- Mises à jour : digests `rust:1.99-bookworm` ([#328](https://github.com/mairie360/Core_API/pull/328), [#332](https://github.com/mairie360/Core_API/pull/332), [#334](https://github.com/mairie360/Core_API/pull/334)), `tokio` 1.54.1 ([#333](https://github.com/mairie360/Core_API/pull/333)), et épinglage de mairie360/CICD de v4.4.0 à v4.4.3 ([#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330)).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/auth/logout` (nouveau, public) : ferme la session désignée par son refresh token, sans JWT, y compris quand le JWT d'accès a expiré. Répond toujours `204` (token inconnu, déjà révoqué ou valide), `400` si le corps est mal formé, `429` au-delà de 60 appels par minute et par adresse, et `503` si la révocation n'a pas pu être publiée sur Redis (à réessayer). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (nouveau, public) : publie les clés publiques des JWT de session au format JWKS, jamais la clé privée. Le tableau `keys` est vide tant que l'instance signe en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif d'un compte via `anonymize_user()`, avec révocation de ses sessions. Répond `404` pour un utilisateur inconnu et `409` si le compte est celui de l'administrateur initial ou s'il n'existe aucun autre administrateur actif pour reprendre groupes, projets et événements. Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions`, `keycloak_account_deleted`. Avec un client Keycloak confidentiel, le compte Keycloak est désactivé avant l'effacement, réactivé si Core refuse, puis supprimé ; `502` si seule cette dernière étape échoue. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export des données de l'utilisateur tel que renvoyé par `export_user_data()`, `404` pour un utilisateur inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Variable `DIRECTORY_GUEST_ACCESS=none` : ferme l'annuaire aux agents qui n'ont que le rôle Guest. `GET /api/v1/user/` et `GET /api/v1/user/{id}/` d'un autre utilisateur répondent alors `403`, leur propre fiche reste lisible. Valeur par défaut `all`, donc sans effet tant qu'elle n'est pas posée. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tout agent (il était masqué). Un compte archivé ne répond plus qu'à lui-même et aux administrateurs, `404` pour les autres. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- `GET /api/v1/roles/` et les routes de l'annuaire restent ouvertes aux cinq rôles ; seul le réglage `DIRECTORY_GUEST_ACCESS=none` ferme l'annuaire aux agents Guest. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Corrections
- Dates des sessions (`GET /api/v1/sessions/`, `GET /api/v1/sessions/history`, `GET /api/v1/admin/users/{userId}/`) : avant, `created_at`, `expires_at` et `revoked_at` étaient au format `2026-09-16 08:42:11 UTC`, que `SessionList` ne sait pas lire : les paramètres et l'administration affichaient « Date indisponible » ou « État indéterminé » et le bouton de révocation n'apparaissait pas. Maintenant, ce sont des dates ISO 8601 UTC à la seconde (`2026-09-16T08:42:11Z`). Le type reste `string`. (MAIR-767, [#354](https://github.com/mairie360/Core_API/pull/354))
- Rôle Admin : avant, `PATCH` ou `PUT /api/v1/admin/roles/{id}` avec `can_be_deleted: true` retirait la protection du rôle système `Admin`, qui pouvait ensuite être renommé ou supprimé, laissant l'instance sans administrateur. Maintenant, la requête répond `403` (`System roles stay protected: can_be_deleted cannot be changed.`) et un rôle personnalisé protégé le reste. Sur PATCH, `null` est traité comme un champ absent. (MAIR-678, [#355](https://github.com/mairie360/Core_API/pull/355))
- Dernier administrateur actif : avant, `POST /api/v1/admin/users/{id}/roles/`, `DELETE /api/v1/admin/users/{id}/roles/{roleId}` et `DELETE /api/v1/admin/users/{id}/` pouvaient retirer le dernier administrateur. Maintenant, ces écritures répondent `409` (`This user is the last active administrator: give the Admin role to another active user first.`) et ne modifient rien. (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352))

#### Technique
- Noms d'opération : le nouveau handler de `POST /api/v1/auth/logout` porte `operation_id = "logout_with_refresh_token"`, pour que le client généré garde `logout()` sur `POST /api/v1/sessions/logout` (le nouveau est `logoutWithRefreshToken()`). Test `operation_ids_test.rs` ajouté. (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349))
- Clés de session : `session_jwt.rs` délègue à `mairie360_api_lib` (4.3.0) ; variables `JWT_PRIVATE_KEY` et `JWT_PUBLIC_KEY` dans les compose, `stack_secrets.sh` génère une paire par run. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Logs : les spans de requête n'emportent plus la query string ni le `Debug` des erreurs (mise à jour de `mairie360_api_lib` en 3.0.1 et tests du marqueur RGPD). (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Matrice d'accès `access-matrix.yaml` et test qui vérifie chaque opération de l'OpenAPI (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274)) ; tests d'authentification RGPD sur les refus de connexion et la révocation après archivage (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268)) ; tests de rate limit derrière BFF (MAIR-679, [#342](https://github.com/mairie360/Core_API/pull/342)). Aucun changement de code métier pour ces trois.
- Image d'exécution des APIs passée en distroless Debian 13 et builder en trixie, avec test `dockerfile_test.rs` (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)).
- Simulation d'usage RGPD, checklist RGPD des PR et règle `ignore` de cargo audit (MAIR-497, MAIR-295, [#273](https://github.com/mairie360/Core_API/pull/273), [#272](https://github.com/mairie360/Core_API/pull/272), [#265](https://github.com/mairie360/Core_API/pull/265)).
- Mises à jour : digests `rust:1.99-bookworm` ([#328](https://github.com/mairie360/Core_API/pull/328), [#332](https://github.com/mairie360/Core_API/pull/332), [#334](https://github.com/mairie360/Core_API/pull/334)), `tokio` 1.54.1 ([#333](https://github.com/mairie360/Core_API/pull/333)), et épinglage de mairie360/CICD de v4.4.0 à v4.4.3 ([#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330)).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
