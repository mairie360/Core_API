# Journal des releases — Core_API

<!-- release-journal-state {"dev": "e40686f91ad0f3ea44c6ccc340a02d60adecea42", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-01 20:15 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `e40686f` | 2026-11-01 20:15 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/admin/users/{userId}/erase` efface un compte pour de bon (droit à l'effacement) : il appelle `anonymize_user()` de la base, publie les sessions révoquées et gère le compte Keycloak (désactivé avant, réactivé si Core refuse, supprimé après ; 502 avec un message explicite si seule cette dernière étape échoue). Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions` (nombre), `keycloak_account_deleted`. Répond 404 si l'utilisateur est inconnu, 409 si le compte est l'administrateur initial ou s'il n'existe pas d'autre administrateur actif pour reprendre ses objets ; effacer deux fois répond `already_anonymized: true`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient les données de l'utilisateur telles que produites par `export_user_data()` (droits d'accès et de portabilité) ; 404 si le compte est inconnu. Le diff est tronqué, la forme exacte de la réponse est à vérifier. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Annuaire fermable aux agents Guest : avec `DIRECTORY_GUEST_ACCESS=none`, `GET /api/v1/user/` et la fiche d'un autre utilisateur répondent 403 aux agents qui n'ont que le rôle Guest, qui gardent leur propre fiche. Défaut `all`, comportement inchangé. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tous les agents, alors qu'il était masqué aux autres. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Journaux des requêtes : la query string (par exemple un e-mail saisi dans une recherche) et la valeur des erreurs (jeton de première connexion, valeur refusée par serde) ne sont plus écrits ; les erreurs de base sont décrites par SQLSTATE, contrainte, table et colonne au lieu du message Postgres. Les logs Keycloak ne nomment plus le compte par son e-mail. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- Image d'exécution passée en distroless Debian 13, avec builder et image de développement sur la même version de Debian, et un test de garde qui échoue si une image n'est pas en Debian 13 ; un pin CICD et les dépendances sont mis à jour (Renovate : digests `rust`, `tokio` 1.54.1). (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339) ; [#328](https://github.com/mairie360/Core_API/pull/328), [#332](https://github.com/mairie360/Core_API/pull/332), [#333](https://github.com/mairie360/Core_API/pull/333), [#334](https://github.com/mairie360/Core_API/pull/334))
- Tests et outillage : matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI, tests RGPD d'authentification, marqueur de logs et simulation d'usage, tests de rate limit via BFF, checklist RGPD sur les PR, pins CICD v4.4.1 à v4.4.3, base de test `dev-76437e3` et advisory RUSTSEC-2026-0387 ignorée. (MAIR-287, MAIR-288, MAIR-290, MAIR-295, MAIR-497, MAIR-612, MAIR-679 : [#268](https://github.com/mairie360/Core_API/pull/268), [#274](https://github.com/mairie360/Core_API/pull/274), [#265](https://github.com/mairie360/Core_API/pull/265), [#273](https://github.com/mairie360/Core_API/pull/273), [#272](https://github.com/mairie360/Core_API/pull/272), [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330), [#342](https://github.com/mairie360/Core_API/pull/342))

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/admin/users/{userId}/erase` efface un compte pour de bon (droit à l'effacement) : il appelle `anonymize_user()` de la base, publie les sessions révoquées et gère le compte Keycloak (désactivé avant, réactivé si Core refuse, supprimé après ; 502 avec un message explicite si seule cette dernière étape échoue). Réponse : `user_id`, `already_anonymized`, `handed_over_to`, `deleted_private_events`, `revoked_sessions` (nombre), `keycloak_account_deleted`. Répond 404 si l'utilisateur est inconnu, 409 si le compte est l'administrateur initial ou s'il n'existe pas d'autre administrateur actif pour reprendre ses objets ; effacer deux fois répond `already_anonymized: true`. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` renvoient les données de l'utilisateur telles que produites par `export_user_data()` (droits d'accès et de portabilité) ; 404 si le compte est inconnu. Le diff est tronqué, la forme exacte de la réponse est à vérifier. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- Annuaire fermable aux agents Guest : avec `DIRECTORY_GUEST_ACCESS=none`, `GET /api/v1/user/` et la fiche d'un autre utilisateur répondent 403 aux agents qui n'ont que le rôle Guest, qui gardent leur propre fiche. Défaut `all`, comportement inchangé. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` : le numéro de téléphone d'un agent est désormais visible de tous les agents, alors qu'il était masqué aux autres. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))
- Journaux des requêtes : la query string (par exemple un e-mail saisi dans une recherche) et la valeur des erreurs (jeton de première connexion, valeur refusée par serde) ne sont plus écrits ; les erreurs de base sont décrites par SQLSTATE, contrainte, table et colonne au lieu du message Postgres. Les logs Keycloak ne nomment plus le compte par son e-mail. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- Image d'exécution passée en distroless Debian 13, avec builder et image de développement sur la même version de Debian, et un test de garde qui échoue si une image n'est pas en Debian 13 ; un pin CICD et les dépendances sont mis à jour (Renovate : digests `rust`, `tokio` 1.54.1). (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339) ; [#328](https://github.com/mairie360/Core_API/pull/328), [#332](https://github.com/mairie360/Core_API/pull/332), [#333](https://github.com/mairie360/Core_API/pull/333), [#334](https://github.com/mairie360/Core_API/pull/334))
- Tests et outillage : matrice d'accès `access-matrix.yaml` vérifiée contre l'OpenAPI, tests RGPD d'authentification, marqueur de logs et simulation d'usage, tests de rate limit via BFF, checklist RGPD sur les PR, pins CICD v4.4.1 à v4.4.3, base de test `dev-76437e3` et advisory RUSTSEC-2026-0387 ignorée. (MAIR-287, MAIR-288, MAIR-290, MAIR-295, MAIR-497, MAIR-612, MAIR-679 : [#268](https://github.com/mairie360/Core_API/pull/268), [#274](https://github.com/mairie360/Core_API/pull/274), [#265](https://github.com/mairie360/Core_API/pull/265), [#273](https://github.com/mairie360/Core_API/pull/273), [#272](https://github.com/mairie360/Core_API/pull/272), [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330), [#342](https://github.com/mairie360/Core_API/pull/342))

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
