# Journal des releases — Core_API

<!-- release-journal-state {"dev": "f3229f1dc1e56f4d8e945314fc44999ee156c773", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-10-31 01:52 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `f3229f1` | 2026-10-31 01:52 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/admin/users/{userId}/erase` (admin) efface un compte pour de bon (droit à l'effacement, art. 17) : il appelle `anonymize_user()` de la base, révoque les sessions via la liste de révocation, puis désactive puis supprime le compte Keycloak. Réponse 200 : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions` (nombre), `keycloak_account_deleted`. 404 si l'utilisateur est inconnu ; 409 si c'est l'administrateur initial ou s'il n'existe aucun autre administrateur actif pour reprendre groupes, projets et événements ; un second appel répond `already_anonymized: true` ; un échec seulement sur la suppression Keycloak répond 502. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` (tout utilisateur connecté, pour ses propres données) et `GET /api/v1/admin/users/{userId}/export` (admin) renvoient toutes les lignes liées au compte, sans les identifiants (droits d'accès et de portabilité, art. 15 et 20). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Option `DIRECTORY_GUEST_ACCESS=none` : les agents qui n'ont que le rôle Guest reçoivent 403 sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur, et gardent leur propre fiche. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tous les agents (décision de la mairie) ; un compte archivé reste invisible, sauf pour lui-même et les administrateurs. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Corrections
- Journaux des requêtes : avant, la query string (ex. un e-mail recherché dans `?search=`), le `Debug` de l'erreur (token de première connexion, valeur refusée par la désérialisation) et l'e-mail des comptes Keycloak étaient journalisés ; maintenant, seuls le type d'erreur et le contexte apparaissent. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Image Docker de l'API : avant, l'image d'exécution était distroless Debian 12, avec une `libssl3` non corrigée (CVE-2026-84782) ; maintenant elle est distroless Debian 13, avec le builder sur trixie, et Trivy ne remonte plus de vulnérabilité HIGH/CRITICAL corrigeable. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))

#### Technique
- Tests : matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI (`tests/endpoints/access_matrix.rs`, MAIR-288) ; tests d'authentification RGPD et d'archivage avec révocation des sessions (MAIR-287) ; test de régression Debian 13 (`tests/dockerfile_test.rs`, MAIR-741) ; marqueur RGPD et simulation d'usage avec scripts et compose dédiés (MAIR-290, MAIR-497, [#273](https://github.com/mairie360/Core_API/pull/273)).
- Dépendances, CI et base : images compose et `TEST_DB_VERSION` passées à `dev-76437e3` (fonctions `anonymize_user()` et `export_user_data()`), `yaml_serde` ajouté, feature `tracing-actix` activée, exception RUSTSEC-2026-0387 dans `audit.toml`, digests rust et tokio 1.54.1 mis à jour par Renovate, workflows de checklist RGPD des PR (MAIR-295, [#272](https://github.com/mairie360/Core_API/pull/272)), et CICD passé en v4.4.3 dans les workflows de promotion (MAIR-612, [#330](https://github.com/mairie360/Core_API/pull/330)).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/admin/users/{userId}/erase` (admin) efface un compte pour de bon (droit à l'effacement, art. 17) : il appelle `anonymize_user()` de la base, révoque les sessions via la liste de révocation, puis désactive puis supprime le compte Keycloak. Réponse 200 : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions` (nombre), `keycloak_account_deleted`. 404 si l'utilisateur est inconnu ; 409 si c'est l'administrateur initial ou s'il n'existe aucun autre administrateur actif pour reprendre groupes, projets et événements ; un second appel répond `already_anonymized: true` ; un échec seulement sur la suppression Keycloak répond 502. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` (tout utilisateur connecté, pour ses propres données) et `GET /api/v1/admin/users/{userId}/export` (admin) renvoient toutes les lignes liées au compte, sans les identifiants (droits d'accès et de portabilité, art. 15 et 20). (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Option `DIRECTORY_GUEST_ACCESS=none` : les agents qui n'ont que le rôle Guest reçoivent 403 sur `GET /api/v1/user/` et sur la fiche d'un autre utilisateur, et gardent leur propre fiche. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tous les agents (décision de la mairie) ; un compte archivé reste invisible, sauf pour lui-même et les administrateurs. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Corrections
- Journaux des requêtes : avant, la query string (ex. un e-mail recherché dans `?search=`), le `Debug` de l'erreur (token de première connexion, valeur refusée par la désérialisation) et l'e-mail des comptes Keycloak étaient journalisés ; maintenant, seuls le type d'erreur et le contexte apparaissent. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))
- Image Docker de l'API : avant, l'image d'exécution était distroless Debian 12, avec une `libssl3` non corrigée (CVE-2026-84782) ; maintenant elle est distroless Debian 13, avec le builder sur trixie, et Trivy ne remonte plus de vulnérabilité HIGH/CRITICAL corrigeable. (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339))

#### Technique
- Tests : matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI (`tests/endpoints/access_matrix.rs`, MAIR-288) ; tests d'authentification RGPD et d'archivage avec révocation des sessions (MAIR-287) ; test de régression Debian 13 (`tests/dockerfile_test.rs`, MAIR-741) ; marqueur RGPD et simulation d'usage avec scripts et compose dédiés (MAIR-290, MAIR-497, [#273](https://github.com/mairie360/Core_API/pull/273)).
- Dépendances, CI et base : images compose et `TEST_DB_VERSION` passées à `dev-76437e3` (fonctions `anonymize_user()` et `export_user_data()`), `yaml_serde` ajouté, feature `tracing-actix` activée, exception RUSTSEC-2026-0387 dans `audit.toml`, digests rust et tokio 1.54.1 mis à jour par Renovate, workflows de checklist RGPD des PR (MAIR-295, [#272](https://github.com/mairie360/Core_API/pull/272)), et CICD passé en v4.4.3 dans les workflows de promotion (MAIR-612, [#330](https://github.com/mairie360/Core_API/pull/330)).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
