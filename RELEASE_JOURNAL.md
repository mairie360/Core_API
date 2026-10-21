# Journal des releases — Core_API

<!-- release-journal-state {"dev": "e4b0ee030f39d1e11a242cfb2ce79d49ef0324cd", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-10-21 08:33 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `e4b0ee0` | 2026-10-21 08:33 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Changements de comportement

- Le déploiement n'est plus automatique depuis `main` : un push sur `main` ne livre que dev (`cicd.yml` s'arrête à `release-dev`). Staging et prod se lancent à la main avec `gh workflow run promote-staging.yml -R mairie360/Core_API [-f sha=<sha>]` puis `promote-prod.yml`, avec l'approbation des environnements `Staging` et `Prod`. Staging refuse un commit sans image `dev-<sha>` ni run vert sur `main` ; prod refuse un commit sans `staging-<sha>`. (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322))
- Les stacks Docker Compose locales (`docker-compose.yml`, `-integration`, `-performance`, `-security`) connectent `core` avec le rôle Postgres `core_api` au lieu du superuser : une requête hors des grants de ce rôle échoue désormais en local. Postgres, Liquibase et le seeder gardent le superuser. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- Bibliothèque `mairie360_api_lib` passée de 3.4.0 à 4.2.0 (dépendance et dev-dependency). La description de #299 cite 4.1.0, mais c'est le diff qui fait foi. Selon cette description, la lib 4.x n'écrit plus dans Redis sans TTL (MAIR-499). (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))

#### Technique

- Workflows CICD `mairie360/CICD` mis à jour de v4.2.1 à v4.4.2 : journal de release rédigé par Claude (v4.3.x), découpage staging/prod (v4.4.0), correction de la création du tag et de la release par `promote-prod` (v4.4.2, HTTP 403 en v4.4.1). Renovate regroupe désormais les `cicd_version` des trois workflows.
- Dépendances et tests : `utoipa-swagger-ui` 10.0.2 et `tokio` 1.54.0 (diffs absents), `rustls-webpki` et `webpki-roots` relevés pour `cargo audit`, trois advisories `zip` ignorées dans `.cargo/audit.toml`, tests d'intégration exécutés sous le rôle `core_api`.

## Différence dev → staging (en dev, pas encore en staging)

#### Technique

- Workflows `cicd.yml`, `promote-staging.yml` et `promote-prod.yml` passés à mairie360/CICD v4.4.1 puis v4.4.2 : la v4.4.2 corrige la création du tag et de la release par `promote-prod` (HTTP 403 de l'API releases en v4.4.1), selon la description de #327. (MAIR-612, [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327))

## Différence staging → prod (en staging, pas encore en prod)

#### Changements de comportement

- Le déploiement n'est plus automatique depuis `main` : un push sur `main` ne livre que dev (`cicd.yml` s'arrête à `release-dev`). Staging et prod se lancent à la main avec `gh workflow run promote-staging.yml -R mairie360/Core_API [-f sha=<sha>]` puis `promote-prod.yml`, avec l'approbation des environnements `Staging` et `Prod`. Staging refuse un commit sans image `dev-<sha>` ni run vert sur `main` ; prod refuse un commit sans `staging-<sha>`. (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322))
- Les stacks Docker Compose locales connectent `core` avec le rôle Postgres `core_api` au lieu du superuser : une requête hors des grants de ce rôle échoue désormais en local. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- Bibliothèque `mairie360_api_lib` passée de 3.4.0 à 4.2.0 (dépendance et dev-dependency). La description de #299 cite 4.1.0, mais c'est le diff qui fait foi. Selon cette description, la lib 4.x n'écrit plus dans Redis sans TTL (MAIR-499). (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))

#### Technique

- Workflows CICD `mairie360/CICD` mis à jour de v4.2.1 à v4.4.0 (renovate, puis découpage staging/prod), avec le journal de release rédigé par Claude (v4.3.x). Renovate regroupe désormais les `cicd_version` des trois workflows.
- Dépendances et tests : `utoipa-swagger-ui` 10.0.2 et `tokio` 1.54.0 (diffs absents), `rustls-webpki` et `webpki-roots` relevés pour `cargo audit`, trois advisories `zip` ignorées dans `.cargo/audit.toml`, tests d'intégration exécutés sous le rôle `core_api`.

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
