# Journal des releases — Core_API

<!-- release-journal-state {"dev": "f3094229bdfcdac09fff8a01a3770db60666d11c", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-01 21:13 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `f309422` | 2026-11-01 21:13 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/admin/users/{userId}/erase` (admin) efface un compte pour de bon (droit à l'effacement) : identité vidée, appartenances, sessions, préférences et événements privés supprimés, groupes, projets et événements transférés à un administrateur actif. Réponse : `{ user_id, already_anonymized, handed_over_to, deleted_private_events, revoked_sessions, keycloak_account_deleted }`. Erreur 404 si l'utilisateur est inconnu, 409 pour l'administrateur initial ou s'il n'existe aucun autre administrateur actif, et `already_anonymized: true` en cas de deuxième appel. Avec Keycloak, le compte est désactivé avant l'effacement, réactivé si Core refuse, puis supprimé ; un échec de la dernière étape renvoie 502 avec un message explicite (d'après la description de la PR). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` (utilisateur connecté) et `GET /api/v1/admin/users/{userId}/export` (admin) renvoient toutes les données liées à l'utilisateur, sans les identifiants, telles que `export_user_data()` les produit. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/auth/jwks` (public) publie les clés publiques Ed25519 au format JWKS, sans partie privée, avec `Cache-Control: public, max-age=300`. Ce n'est qu'informatif : aucun service ne l'interroge à l'exécution. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Avec `DIRECTORY_GUEST_ACCESS=none` (défaut : `all`), les agents qui n'ont que le rôle Guest reçoivent 403 sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur ; ils lisent toujours leur propre fiche. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- Les JWT de session sont signés en EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`) dès que `JWT_PRIVATE_KEY` est défini, pour la connexion, la connexion Keycloak, la réinitialisation du mot de passe et le refresh. Les tokens HS256 restent acceptés tant que `JWT_SECRET` est défini ; sans clé privée, Core continue de signer en HS256. Les autres APIs et BFFs doivent recevoir `JWT_PUBLIC_KEY`. Les environnements de test génèrent une paire par exécution ; la compose de dev n'en a pas et reste en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `GET /api/v1/user/{id}/` montre désormais le téléphone d'un agent à tout agent (usage professionnel). Un compte archivé n'est visible que par lui-même et par les administrateurs. Le test `other_users_record_hides_phone_and_archived_accounts` est devenu `other_users_record_shows_phone_and_hides_archived_accounts`. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Corrections
- Déconnexion avec un JWT expiré : avant, la déconnexion passait par des routes protégées par JWT, un JWT expiré recevait 401 et le refresh token restait valide 30 jours côté serveur. Maintenant, `POST /api/v1/auth/logout` (public, corps `{ refresh_token }`) révoque la session du refresh token, publie la révocation et répond toujours 204, même si le token est inconnu, déjà révoqué ou valide. Il répond 400 pour un corps invalide, 429 au-delà de 60 par minute et par IP, et 503 si la publication Redis échoue (un nouvel appel la refait). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))

#### Technique
- Mise à jour de `mairie360_api_lib` (4.2.0 vers 4.3.0) et de sa version 3.0.1 pour des journaux sans données personnelles ; les requêtes HTTP journalisées n'incluent plus la query string ni le `Debug` des erreurs (MAIR-680, MAIR-290, [#346](https://github.com/mairie360/Core_API/pull/346), [#265](https://github.com/mairie360/Core_API/pull/265)).
- Images Docker passées sur Debian 13 (distroless `cc-debian13`, `rust:1.99-trixie`), avec un test qui échoue si une image n'est pas sur Debian 13 (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)).
- Matrice d'accès `access-matrix.yaml` et tests associés qui vérifient chaque opération de l'OpenAPI (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274)) ; tests de limitation de débit derrière un BFF (MAIR-679, [#342](https://github.com/mairie360/Core_API/pull/342)) ; tests d'authentification et de révocation (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268)).
- Stacks de test RGPD : marqueur de journaux (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265)) et simulation d'usage (MAIR-497, [#273](https://github.com/mairie360/Core_API/pull/273)) ; checklist RGPD sur les PR (MAIR-295, [#272](https://github.com/mairie360/Core_API/pull/272)).
- Images de base de données `dev-76437e3` dans les compose et `.cargo/config.toml` (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277)).
- Mises à jour CICD v4.4.0 vers v4.4.3 dans les workflows (MAIR-612, [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330)) ; digests `rust:1.99` mis à jour (renovate, [#328](https://github.com/mairie360/Core_API/pull/328), [#332](https://github.com/mairie360/Core_API/pull/332), [#334](https://github.com/mairie360/Core_API/pull/334)) ; tokio 1.54.1 ([#333](https://github.com/mairie360/Core_API/pull/333), diff absent).
- Ajustements `.gitleaksignore` et `cargo audit` (RUSTSEC-2026-0387) pour les faux positifs et la dépendance `crypto-bigint`.

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/admin/users/{userId}/erase` (admin) efface un compte pour de bon (droit à l'effacement) : identité vidée, appartenances, sessions, préférences et événements privés supprimés, groupes, projets et événements transférés à un administrateur actif. Réponse : `{ user_id, already_anonymized, handed_over_to, deleted_private_events, revoked_sessions, keycloak_account_deleted }`. Erreur 404 si l'utilisateur est inconnu, 409 pour l'administrateur initial ou s'il n'existe aucun autre administrateur actif, et `already_anonymized: true` en cas de deuxième appel. Avec Keycloak, le compte est désactivé avant l'effacement, réactivé si Core refuse, puis supprimé ; un échec de la dernière étape renvoie 502 avec un message explicite (d'après la description de la PR). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` (utilisateur connecté) et `GET /api/v1/admin/users/{userId}/export` (admin) renvoient toutes les données liées à l'utilisateur, sans les identifiants, telles que `export_user_data()` les produit. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/auth/jwks` (public) publie les clés publiques Ed25519 au format JWKS, sans partie privée, avec `Cache-Control: public, max-age=300`. Ce n'est qu'informatif : aucun service ne l'interroge à l'exécution. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- Avec `DIRECTORY_GUEST_ACCESS=none` (défaut : `all`), les agents qui n'ont que le rôle Guest reçoivent 403 sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur ; ils lisent toujours leur propre fiche. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- Les JWT de session sont signés en EdDSA (`kid`, `iss` `mairie360-core`, `aud` `mairie360`) dès que `JWT_PRIVATE_KEY` est défini, pour la connexion, la connexion Keycloak, la réinitialisation du mot de passe et le refresh. Les tokens HS256 restent acceptés tant que `JWT_SECRET` est défini ; sans clé privée, Core continue de signer en HS256. Les autres APIs et BFFs doivent recevoir `JWT_PUBLIC_KEY`. Les environnements de test génèrent une paire par exécution ; la compose de dev n'en a pas et reste en HS256. (MAIR-680, [#346](https://github.com/mairie360/Core_API/pull/346))
- `GET /api/v1/user/{id}/` montre désormais le téléphone d'un agent à tout agent (usage professionnel). Un compte archivé n'est visible que par lui-même et par les administrateurs. Le test `other_users_record_hides_phone_and_archived_accounts` est devenu `other_users_record_shows_phone_and_hides_archived_accounts`. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Corrections
- Déconnexion avec un JWT expiré : avant, la déconnexion passait par des routes protégées par JWT, un JWT expiré recevait 401 et le refresh token restait valide 30 jours côté serveur. Maintenant, `POST /api/v1/auth/logout` (public, corps `{ refresh_token }`) révoque la session du refresh token, publie la révocation et répond toujours 204, même si le token est inconnu, déjà révoqué ou valide. Il répond 400 pour un corps invalide, 429 au-delà de 60 par minute et par IP, et 503 si la publication Redis échoue (un nouvel appel la refait). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))

#### Technique
- Mise à jour de `mairie360_api_lib` (4.2.0 vers 4.3.0) et de sa version 3.0.1 pour des journaux sans données personnelles ; les requêtes HTTP journalisées n'incluent plus la query string ni le `Debug` des erreurs (MAIR-680, MAIR-290, [#346](https://github.com/mairie360/Core_API/pull/346), [#265](https://github.com/mairie360/Core_API/pull/265)).
- Images Docker passées sur Debian 13 (distroless `cc-debian13`, `rust:1.99-trixie`), avec un test qui échoue si une image n'est pas sur Debian 13 (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)).
- Matrice d'accès `access-matrix.yaml` et tests associés qui vérifient chaque opération de l'OpenAPI (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274)) ; tests de limitation de débit derrière un BFF (MAIR-679, [#342](https://github.com/mairie360/Core_API/pull/342)) ; tests d'authentification et de révocation (MAIR-287, [#268](https://github.com/mairie360/Core_API/pull/268)).
- Stacks de test RGPD : marqueur de journaux (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265)) et simulation d'usage (MAIR-497, [#273](https://github.com/mairie360/Core_API/pull/273)) ; checklist RGPD sur les PR (MAIR-295, [#272](https://github.com/mairie360/Core_API/pull/272)).
- Images de base de données `dev-76437e3` dans les compose et `.cargo/config.toml` (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277)).
- Mises à jour CICD v4.4.0 vers v4.4.3 dans les workflows (MAIR-612, [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330)) ; digests `rust:1.99` mis à jour (renovate, [#328](https://github.com/mairie360/Core_API/pull/328), [#332](https://github.com/mairie360/Core_API/pull/332), [#334](https://github.com/mairie360/Core_API/pull/334)) ; tokio 1.54.1 ([#333](https://github.com/mairie360/Core_API/pull/333), diff absent).
- Ajustements `.gitleaksignore` et `cargo audit` (RUSTSEC-2026-0387) pour les faux positifs et la dépendance `crypto-bigint`.

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
