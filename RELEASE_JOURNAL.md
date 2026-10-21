# Journal des releases — Core_API

<!-- release-journal-state {"dev": "19adc1ecfc155e1665e35c68f6b1ed6a7dc26c5d", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-10-21 03:50 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `19adc1e` | 2026-10-21 03:50 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Changements de comportement
- Un push sur `main` ne déclenche plus que la release dev : `cicd.yml` s'arrête à `release-dev`. Staging et prod se promeuvent désormais à la main avec `promote-staging.yml` puis `promote-prod.yml` (`gh workflow run ... -R mairie360/Core_API [-f sha=<sha>]`), avec l'approbation des environnements `Staging` et `Prod`. Staging refuse un commit sans image `dev-<sha>` et sans run main vert ; prod refuse un commit sans image `staging-<sha>`. (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322))

#### Technique
- Pin de mairie360/CICD porté de v4.2.1 à v4.4.1 via Renovate et les PR de MAIR-571 et MAIR-612 : v4.2.2, v4.3.0 à v4.3.3 (dont l'étape de journal des releases rédigé par Claude), puis v4.4.0 et v4.4.1 (([#310](https://github.com/mairie360/Core_API/pull/310), [#314](https://github.com/mairie360/Core_API/pull/314), [#315](https://github.com/mairie360/Core_API/pull/315), [#318](https://github.com/mairie360/Core_API/pull/318), [#320](https://github.com/mairie360/Core_API/pull/320), [#322](https://github.com/mairie360/Core_API/pull/322), [#325](https://github.com/mairie360/Core_API/pull/325))).
- Dépendances Rust : `mairie360_api_lib` passe de 3.4.0 à 4.2.0 dans `Cargo.toml` (la description de la PR indique que la lib 4.x n'écrit plus dans Redis sans TTL, MAIR-499) ([#299](https://github.com/mairie360/Core_API/pull/299)) ; `tokio` 1.53.3 vers 1.54.0 ([#316](https://github.com/mairie360/Core_API/pull/316)) ; `utoipa-swagger-ui` 10.0.1 vers 10.0.2 ([#319](https://github.com/mairie360/Core_API/pull/319)).
- Audit de sécurité : `rustls-webpki` en 0.103.16 et `webpki-roots` / `webpki-root-certs` en 1.0.10 pour débloquer `cargo audit` ([#322](https://github.com/mairie360/Core_API/pull/322)) ; ignore de RUSTSEC-2026-0367, 0368 et 0369 (zip, build-dependency de utoipa-swagger-ui) dans `.cargo/audit.toml` ([#299](https://github.com/mairie360/Core_API/pull/299)).
- Tests d'intégration et `docker-compose*.yml` : Core s'exécute avec le rôle Postgres `core_api` au lieu du superuser, Postgres, Liquibase et le seeder gardant le superuser ([#299](https://github.com/mairie360/Core_API/pull/299), diff tronqué).

## Différence dev → staging (en dev, pas encore en staging)

#### Technique
- Pin de mairie360/CICD porté de v4.4.0 à v4.4.1 dans `cicd.yml`, `promote-staging.yml` et `promote-prod.yml`, pour corriger le dry run de semantic-release de `promote-prod` (MAIR-612, [#325](https://github.com/mairie360/Core_API/pull/325)). Aucun changement fonctionnel.

## Différence staging → prod (en staging, pas encore en prod)

#### Changements de comportement
- Un push sur `main` ne déclenche plus que la release dev : `cicd.yml` s'arrête à `release-dev`. Staging et prod se promeuvent désormais à la main avec `promote-staging.yml` puis `promote-prod.yml` (`gh workflow run ... -R mairie360/Core_API [-f sha=<sha>]`), avec l'approbation des environnements `Staging` et `Prod`. Staging refuse un commit sans image `dev-<sha>` et sans run main vert ; prod refuse un commit sans image `staging-<sha>`. (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322))

#### Technique
- Pin de mairie360/CICD porté de v4.2.1 à v4.4.0 via Renovate et les PR de MAIR-571 et MAIR-494 : v4.2.2, v4.3.0 à v4.3.3 (dont l'étape de journal des releases rédigé par Claude), puis v4.4.0 ([#310](https://github.com/mairie360/Core_API/pull/310), [#314](https://github.com/mairie360/Core_API/pull/314), [#315](https://github.com/mairie360/Core_API/pull/315), [#318](https://github.com/mairie360/Core_API/pull/318), [#320](https://github.com/mairie360/Core_API/pull/320), [#322](https://github.com/mairie360/Core_API/pull/322)).
- Dépendances Rust : `mairie360_api_lib` passe de 3.4.0 à 4.2.0 dans `Cargo.toml` (la description de la PR indique que la lib 4.x n'écrit plus dans Redis sans TTL, MAIR-499) ([#299](https://github.com/mairie360/Core_API/pull/299)) ; `tokio` 1.53.3 vers 1.54.0 ([#316](https://github.com/mairie360/Core_API/pull/316)) ; `utoipa-swagger-ui` 10.0.1 vers 10.0.2 ([#319](https://github.com/mairie360/Core_API/pull/319)).
- Audit de sécurité : `rustls-webpki` en 0.103.16 et `webpki-roots` / `webpki-root-certs` en 1.0.10 pour débloquer `cargo audit` ([#322](https://github.com/mairie360/Core_API/pull/322)) ; ignore de RUSTSEC-2026-0367, 0368 et 0369 (zip, build-dependency de utoipa-swagger-ui) dans `.cargo/audit.toml` ([#299](https://github.com/mairie360/Core_API/pull/299)).
- Tests d'intégration et `docker-compose*.yml` : Core s'exécute avec le rôle Postgres `core_api` au lieu du superuser, Postgres, Liquibase et le seeder gardant le superuser ([#299](https://github.com/mairie360/Core_API/pull/299), diff tronqué).

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-21 03:50 UTC | dev | — | `19adc1e` | [run](https://github.com/mairie360/Core_API/actions/runs/39539859800) |
| 2026-10-21 03:14 UTC | staging | — | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39528369269) |
| 2026-10-21 01:36 UTC | dev | — | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39526428093) |
| 2026-10-20 17:07 UTC | staging | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:57 UTC | dev | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:49 UTC | dev | — | `acf2207` | [run](https://github.com/mairie360/Core_API/actions/runs/39459275998) |
| 2026-10-19 20:10 UTC | dev | — | `30d9bb5` | [run](https://github.com/mairie360/Core_API/actions/runs/39315745705) |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
