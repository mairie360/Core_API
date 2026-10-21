# Journal des releases — Core_API

<!-- release-journal-state {"dev": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "b9007977b920b85110b7e4b630bec63bf82ef365", "updated": {"dev": "2026-10-21 01:36 UTC", "staging": "2026-10-20 17:07 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `b900797` | 2026-10-20 17:07 UTC |
| dev | — | `cea777d` | 2026-10-21 01:36 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Nouveautés
- Le pipeline de release écrit un journal dans `RELEASE_JOURNAL.md` sur `main` à chaque release dev, staging et prod, avec un résumé rédigé par Claude (MAIR-571, [#314](https://github.com/mairie360/Core_API/pull/314), [#315](https://github.com/mairie360/Core_API/pull/315)).
- Promotion manuelle vers staging et prod via `promote-staging.yml` et `promote-prod.yml` (`workflow_dispatch`, entrée optionnelle `sha`, lancement avec `gh workflow run`) (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)).

#### Changements de comportement
- Un push sur `main` ne va plus jusqu'à staging ni prod : le pipeline s'arrête à `release-dev` (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)).
- Les stacks `docker-compose*.yml` connectent `core` avec le rôle Postgres `core_api` (mot de passe `core-api-password`) au lieu du superuser. Une requête hors des grants de ce rôle échoue désormais en local et en CI avec `permission denied` (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299)).

#### Technique
- `mairie360_api_lib` passe à 4.2.0 dans `Cargo.toml`, avec `tokio` 1.54.0 et `utoipa-swagger-ui` 10.0.2 (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299), [#316](https://github.com/mairie360/Core_API/pull/316), [#319](https://github.com/mairie360/Core_API/pull/319)).
- `.cargo/audit.toml` ignore RUSTSEC-2026-0367, 0368 et 0369 (zip 8, build-dependency de utoipa-swagger-ui). Les pins `mairie360/CICD` passent de v4.2.2 à v4.3.3 via Renovate (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299), [#310](https://github.com/mairie360/Core_API/pull/310), [#318](https://github.com/mairie360/Core_API/pull/318), [#320](https://github.com/mairie360/Core_API/pull/320)).
- Les tests d'intégration passent au rôle `core_api` (`get_pool()` sans URL, `core_db_url()`, `get_raw_pool` pour les fixtures) ; le diff fourni est tronqué, donc la vue est partielle. `CLAUDE.md` est mis à jour (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299)).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- Promotion manuelle vers staging via `promote-staging.yml` (`gh workflow run promote-staging.yml -R mairie360/Core_API [-f sha=<sha>]`). Staging refuse un commit sans image `dev-<sha>` ni run main vert, puis lance les stacks d'intégration, ZAP et k6 sur cette image (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)).

#### Changements de comportement
- Un push sur `main` ne déclenche plus de déploiement staging : staging ne se met à jour que par promotion manuelle (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)).

#### Technique
- `cicd.yml` pin `mairie360/CICD` en v4.4.0. `promote-prod.yml` est ajouté avec le même pin, et `renovate.json` regroupe les trois workflows dans une seule PR (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)).
- Mise à jour Renovate de `mairie360/CICD` vers v4.3.3 (MAIR-571 et suivants, [#320](https://github.com/mairie360/Core_API/pull/320)).

## Différence staging → prod (en staging, pas encore en prod)

#### Nouveautés
- Le pipeline de release écrit `RELEASE_JOURNAL.md` sur `main` pour les releases staging et prod, avec un résumé rédigé par Claude (MAIR-571, [#314](https://github.com/mairie360/Core_API/pull/314), [#315](https://github.com/mairie360/Core_API/pull/315)).

#### Technique
- `mairie360_api_lib` passe à 4.2.0 dans `Cargo.toml`, avec `tokio` 1.54.0 et `utoipa-swagger-ui` 10.0.2 (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299), [#316](https://github.com/mairie360/Core_API/pull/316), [#319](https://github.com/mairie360/Core_API/pull/319)).
- `.cargo/audit.toml` ignore RUSTSEC-2026-0367, 0368 et 0369. Les pins `mairie360/CICD` passent de v4.2.2 à v4.3.3 via Renovate (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299), [#310](https://github.com/mairie360/Core_API/pull/310), [#318](https://github.com/mairie360/Core_API/pull/318), [#320](https://github.com/mairie360/Core_API/pull/320)).
- Tests d'intégration passés au rôle `core_api` et stacks compose alignées, sans changement du code applicatif visible dans le diff fourni, qui est tronqué (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299)).

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-21 01:36 UTC | dev | — | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39526428093) |
| 2026-10-20 17:07 UTC | staging | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:57 UTC | dev | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:49 UTC | dev | — | `acf2207` | [run](https://github.com/mairie360/Core_API/actions/runs/39459275998) |
| 2026-10-19 20:10 UTC | dev | — | `30d9bb5` | [run](https://github.com/mairie360/Core_API/actions/runs/39315745705) |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
