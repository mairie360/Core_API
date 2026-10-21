# Journal des releases — Core_API

<!-- release-journal-state {"dev": "971899000dcd3b3ab46e8f6e29e22964cc4e26a1", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-10-21 09:07 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `9718990` | 2026-10-21 09:07 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Technique
- Dépendances Rust mises à jour : `mairie360_api_lib` passe à 4.2.0 (dépendance et dev-dependency, [#299](https://github.com/mairie360/Core_API/pull/299)), `tokio` à 1.54.0 ([#316](https://github.com/mairie360/Core_API/pull/316)), `utoipa-swagger-ui` à 10.0.2 ([#319](https://github.com/mairie360/Core_API/pull/319)), et l'image `rust:1.99-bookworm` épinglée par digest dans `development.Dockerfile` ([#328](https://github.com/mairie360/Core_API/pull/328)).
- Tests et environnements de développement : les tests et les `docker-compose*.yml` font tourner le code Core avec le rôle `core_api` au lieu du superuser, `get_pool()` ne prend plus d'URL, et `.cargo/audit.toml` ignore RUSTSEC-2026-0367, 0368 et 0369 (zip, build-dependency). (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- Pipeline de release : CICD évolue de v4.2.1 à v4.4.2 via les PR Renovate ([#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#320](https://github.com/mairie360/Core_API/pull/320), [#318](https://github.com/mairie360/Core_API/pull/318)) et MAIR-571 ([#314](https://github.com/mairie360/Core_API/pull/314)) ; ajout des workflows manuels `promote-staging.yml` et `promote-prod.yml` (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)) ; corrections de promotion (MAIR-612, [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327)).

## Différence dev → staging (en dev, pas encore en staging)

#### Technique
- Image de développement : `development.Dockerfile` utilise désormais le digest `5c1a173` de `rust:1.99-bookworm` ([#328](https://github.com/mairie360/Core_API/pull/328)).
- Pipeline : `cicd.yml` et les workflows `promote-*.yml` passent de CICD v4.4.0 à v4.4.1 puis v4.4.2 (MAIR-612, [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327)) ; selon la description de la PR, la v4.4.2 corrige la création du tag et de la release lors du `promote-prod`.

## Différence staging → prod (en staging, pas encore en prod)

#### Technique
- Dépendances Rust mises à jour : `mairie360_api_lib` passe à 4.2.0 ([#299](https://github.com/mairie360/Core_API/pull/299)), `tokio` à 1.54.0 ([#316](https://github.com/mairie360/Core_API/pull/316)) et `utoipa-swagger-ui` à 10.0.2 ([#319](https://github.com/mairie360/Core_API/pull/319)).
- Tests et environnements de développement : les tests et les `docker-compose*.yml` font tourner le code Core avec le rôle `core_api` au lieu du superuser, `get_pool()` ne prend plus d'URL, et `.cargo/audit.toml` ignore RUSTSEC-2026-0367, 0368 et 0369 (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299)).
- Pipeline de release : CICD passe de v4.2.2 à v4.3.3 via les PR Renovate ([#320](https://github.com/mairie360/Core_API/pull/320), [#318](https://github.com/mairie360/Core_API/pull/318)) et MAIR-571 ([#314](https://github.com/mairie360/Core_API/pull/314), [#315](https://github.com/mairie360/Core_API/pull/315)) ; ajout des workflows manuels `promote-staging.yml` et `promote-prod.yml` (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)).

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
