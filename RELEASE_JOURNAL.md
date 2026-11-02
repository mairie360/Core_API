# Journal des releases — Core_API

<!-- release-journal-state {"dev": "6657fe8b18ed48ef651cb3211b312232ad40df65", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-02 16:58 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `6657fe8` | 2026-11-02 16:58 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif d'un compte (art. 17 RGPD). Appelle `anonymize_user()` de la base : identité effacée, appartenances, sessions, préférences et liens SSO supprimés, événements privés supprimés, groupes, projets et événements transférés à un administrateur actif. Les sessions révoquées sont publiées sur la liste de révocation. Avec un client Keycloak confidentiel, le compte est désactivé avant l'effacement, réactivé si Core refuse, puis supprimé. 404 si le compte est inconnu, 409 si l'administrateur est le compte seed ou s'il n'y a aucun repreneur. Un second appel répond `already_anonymized: true`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export des données d'un utilisateur (art. 15 et 20 RGPD), renvoyé tel que produit par `export_user_data()`, sans les identifiants. 404 si le compte est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `POST /api/v1/auth/logout` (public) : déconnexion avec le seul refresh token. Révoque la session et la publie sur la liste de révocation. Répond toujours 204 (token inconnu, déjà révoqué ou valide), 400 si le corps est invalide, 429 au-delà de 60 par minute et par adresse, 503 si la publication Redis échoue. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : clés publiques des JWT de session au format JWKS, avec `Cache-Control` de 5 minutes. Le tableau est vide tant que l'instance signe en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Annuaire : avec `DIRECTORY_GUEST_ACCESS=none`, les agents qui n'ont que le rôle Guest reçoivent 403 sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur ; ils gardent la leur. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le téléphone d'un agent est désormais visible de tout agent (avant : masqué). Les comptes archivés restent visibles seulement à eux-mêmes et aux administrateurs. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Signature des JWT de session : avec `JWT_PRIVATE_KEY`, Core émet des JWT `EdDSA` (`kid`, `iss` `mairie360-core`, `aud` `mairie360`). Sans clé privée, il signe en HS256 comme avant. Les HS256 restent acceptés tant que `JWT_SECRET` est défini. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))

#### Corrections
- Dernier administrateur (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352)) : avant, `POST /api/v1/admin/users/{userId}/roles/`, `DELETE /api/v1/admin/users/{userId}/roles/{roleId}` et `DELETE /api/v1/admin/users/{userId}/` pouvaient retirer le dernier administrateur actif. Maintenant ces opérations répondent 409 (`This user is the last active administrator…`) et ne modifient rien quand aucun administrateur actif ne resterait. Un appelant doit donc donner le rôle Admin à un autre utilisateur actif avant.
- Déconnexion avec un JWT expiré (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344)) : avant, `POST /api/v1/sessions/logout` et `POST /api/v1/sessions/revoke` répondaient 401 et le refresh token restait valide sur le serveur. Maintenant `POST /api/v1/auth/logout` révoque la session sans JWT.
- Journaux (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265)) : avant, la query string (dont une adresse e-mail recherchée), le token de première connexion et des valeurs serde pouvaient apparaître dans les journaux et les traces. Maintenant les erreurs sont décrites par type et contexte, et la query string est retirée du span racine. Les logs Keycloak ne nomment plus le compte par son e-mail.

#### Technique
- Image d'exécution passée en distroless Debian 13 (`cc-debian13`), builder et `development.Dockerfile` sur trixie, avec un test `dockerfile_test.rs` (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)).
- Handler de `POST /api/v1/auth/logout` nommé `logout_with_refresh_token` pour que le client généré garde `logout()` sur `/sessions/logout`, avec `operation_ids_test.rs` (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349)).
- Matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI, tests de rate limit derrière BFF, tests d'authentification et d'archivage (MAIR-288, MAIR-679, MAIR-287 : [#274](https://github.com/mairie360/Core_API/pull/274), [#342](https://github.com/mairie360/Core_API/pull/342), [#268](https://github.com/mairie360/Core_API/pull/268)).
- Checklist RGPD des PR, simulation d'usage RGPD et test du marqueur (MAIR-295, MAIR-497, MAIR-290 : [#272](https://github.com/mairie360/Core_API/pull/272), [#273](https://github.com/mairie360/Core_API/pull/273), [#265](https://github.com/mairie360/Core_API/pull/265)).
- Dépendances : `mairie360_api_lib` 3.0.1 puis 4.2.0 à 4.3.0, image Database `dev-76437e3`, digests `rust:1.99-bookworm` mis à jour par Renovate, `tokio` 1.54.1, et CICD passé de v4.4.0 à v4.4.3 dans les workflows.

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif d'un compte (art. 17 RGPD). Appelle `anonymize_user()` de la base : identité effacée, appartenances, sessions, préférences et liens SSO supprimés, événements privés supprimés, groupes, projets et événements transférés à un administrateur actif. Les sessions révoquées sont publiées sur la liste de révocation. Avec un client Keycloak confidentiel, le compte est désactivé avant l'effacement, réactivé si Core refuse, puis supprimé. 404 si le compte est inconnu, 409 si l'administrateur est le compte seed ou s'il n'y a aucun repreneur. Un second appel répond `already_anonymized: true`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` : export des données d'un utilisateur (art. 15 et 20 RGPD), renvoyé tel que produit par `export_user_data()`, sans les identifiants. 404 si le compte est inconnu. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `POST /api/v1/auth/logout` (public) : déconnexion avec le seul refresh token. Révoque la session et la publie sur la liste de révocation. Répond toujours 204 (token inconnu, déjà révoqué ou valide), 400 si le corps est invalide, 429 au-delà de 60 par minute et par adresse, 503 si la publication Redis échoue. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `GET /api/v1/auth/jwks` (public) : clés publiques des JWT de session au format JWKS, avec `Cache-Control` de 5 minutes. Le tableau est vide tant que l'instance signe en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Annuaire : avec `DIRECTORY_GUEST_ACCESS=none`, les agents qui n'ont que le rôle Guest reçoivent 403 sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur ; ils gardent la leur. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le téléphone d'un agent est désormais visible de tout agent (avant : masqué). Les comptes archivés restent visibles seulement à eux-mêmes et aux administrateurs. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Signature des JWT de session : avec `JWT_PRIVATE_KEY`, Core émet des JWT `EdDSA` (`kid`, `iss` `mairie360-core`, `aud` `mairie360`). Sans clé privée, il signe en HS256 comme avant. Les HS256 restent acceptés tant que `JWT_SECRET` est défini. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))

#### Corrections
- Dernier administrateur (MAIR-678, [#352](https://github.com/mairie360/Core_API/pull/352)) : avant, `POST /api/v1/admin/users/{userId}/roles/`, `DELETE /api/v1/admin/users/{userId}/roles/{roleId}` et `DELETE /api/v1/admin/users/{userId}/` pouvaient retirer le dernier administrateur actif. Maintenant ces opérations répondent 409 (`This user is the last active administrator…`) et ne modifient rien quand aucun administrateur actif ne resterait. Un appelant doit donc donner le rôle Admin à un autre utilisateur actif avant.
- Déconnexion avec un JWT expiré (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344)) : avant, `POST /api/v1/sessions/logout` et `POST /api/v1/sessions/revoke` répondaient 401 et le refresh token restait valide sur le serveur. Maintenant `POST /api/v1/auth/logout` révoque la session sans JWT.
- Journaux (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265)) : avant, la query string (dont une adresse e-mail recherchée), le token de première connexion et des valeurs serde pouvaient apparaître dans les journaux et les traces. Maintenant les erreurs sont décrites par type et contexte, et la query string est retirée du span racine. Les logs Keycloak ne nomment plus le compte par son e-mail.

#### Technique
- Image d'exécution passée en distroless Debian 13 (`cc-debian13`), builder et `development.Dockerfile` sur trixie, avec un test `dockerfile_test.rs` (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)).
- Handler de `POST /api/v1/auth/logout` nommé `logout_with_refresh_token` pour que le client généré garde `logout()` sur `/sessions/logout`, avec `operation_ids_test.rs` (MAIR-692, [#349](https://github.com/mairie360/Core_API/pull/349)).
- Matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI, tests de rate limit derrière BFF, tests d'authentification et d'archivage (MAIR-288, MAIR-679, MAIR-287 : [#274](https://github.com/mairie360/Core_API/pull/274), [#342](https://github.com/mairie360/Core_API/pull/342), [#268](https://github.com/mairie360/Core_API/pull/268)).
- Checklist RGPD des PR, simulation d'usage RGPD et test du marqueur (MAIR-295, MAIR-497, MAIR-290 : [#272](https://github.com/mairie360/Core_API/pull/272), [#273](https://github.com/mairie360/Core_API/pull/273), [#265](https://github.com/mairie360/Core_API/pull/265)).
- Dépendances : `mairie360_api_lib` 3.0.1 puis 4.2.0 à 4.3.0, image Database `dev-76437e3`, digests `rust:1.99-bookworm` mis à jour par Renovate, `tokio` 1.54.1, et CICD passé de v4.4.0 à v4.4.3 dans les workflows.

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
