# Journal des releases — Core_API

<!-- release-journal-state {"dev": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-10-21 01:36 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `cea777d` | 2026-10-21 01:36 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Changements de comportement

- Un push sur `main` ne déclenche plus que la release dev : `cicd.yml` s'arrête à `release-dev`, il n'y a plus de run en attente sur les environnements `Staging` et `Prod`. (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322))
- Staging et prod ne se déploient plus automatiquement : la promotion est manuelle, via `promote-staging.yml` puis `promote-prod.yml` (`workflow_dispatch`, entrée `sha` optionnelle, un passage à la fois par cible). Staging refuse un commit sans image `dev-<sha>` ni run vert sur main ; prod refuse un commit sans `staging-<sha>`. (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322))

#### Technique

- `mairie360/CICD` passe de v4.2.2 à v4.4.0 en cinq PR (#314, #315, #318, #320, #322). Chaque release dev, staging et prod réécrit `RELEASE_JOURNAL.md` sur `main` avec un résumé rédigé par Claude (CICD v4.3.1, #314).
- Dépendances : `tokio` 1.53.3 → 1.54.0 (#316), `utoipa-swagger-ui` 10.0.1 → 10.0.2 (#319), `mairie360_api_lib` 3.4.0 → 4.2.0 dans `Cargo.toml` (#299).
- Tests et compose (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299)) : le code de Core tourne sous le rôle `core_api` et non plus sous le superuser, et `docker-compose*.yml` connecte `core` avec ce rôle. Une requête hors des grants de Database échoue donc en `permission denied` en local et en CI. Le diff de #299 est tronqué : seul le périmètre visible est décrit ici.
- Audit (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)) : `.cargo/audit.toml` ignore RUSTSEC-2026-0367/0368/0369 (zip 8, build-dependency de utoipa-swagger-ui). Selon la description de #322, `rustls-webpki`, `webpki-roots` et `webpki-root-certs` sont montés pour débloquer `cargo audit` (RUSTSEC-2026-0381/0382/0383).

## Différence dev → staging (en dev, pas encore en staging)

_Rien._

## Différence staging → prod (en staging, pas encore en prod)

#### Changements de comportement

- Un push sur `main` ne déclenche plus que la release dev : `cicd.yml` s'arrête à `release-dev`, il n'y a plus de run en attente sur les environnements `Staging` et `Prod`. (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322))
- Staging et prod ne se déploient plus automatiquement : la promotion est manuelle, via `promote-staging.yml` puis `promote-prod.yml` (`workflow_dispatch`, entrée `sha` optionnelle, un passage à la fois par cible). Staging refuse un commit sans image `dev-<sha>` ni run vert sur main ; prod refuse un commit sans `staging-<sha>`. (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322))

#### Technique

- `mairie360/CICD` passe de v4.2.2 à v4.4.0 en cinq PR (#314, #315, #318, #320, #322). Chaque release dev, staging et prod réécrit `RELEASE_JOURNAL.md` sur `main` avec un résumé rédigé par Claude (CICD v4.3.1, #314).
- Dépendances : `tokio` 1.53.3 → 1.54.0 (#316), `utoipa-swagger-ui` 10.0.1 → 10.0.2 (#319), `mairie360_api_lib` 3.4.0 → 4.2.0 dans `Cargo.toml` (#299).
- Tests et compose (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299)) : le code de Core tourne sous le rôle `core_api` et non plus sous le superuser, et `docker-compose*.yml` connecte `core` avec ce rôle. Une requête hors des grants de Database échoue donc en `permission denied` en local et en CI. Le diff de #299 est tronqué : seul le périmètre visible est décrit ici.
- Audit (MAIR-494, [#322](https://github.com/mairie360/Core_API/pull/322)) : `.cargo/audit.toml` ignore RUSTSEC-2026-0367/0368/0369 (zip 8, build-dependency de utoipa-swagger-ui). Selon la description de #322, `rustls-webpki`, `webpki-roots` et `webpki-root-certs` sont montés pour débloquer `cargo audit` (RUSTSEC-2026-0381/0382/0383).

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-21 03:14 UTC | staging | — | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39528369269) |
| 2026-10-21 01:36 UTC | dev | — | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39526428093) |
| 2026-10-20 17:07 UTC | staging | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:57 UTC | dev | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:49 UTC | dev | — | `acf2207` | [run](https://github.com/mairie360/Core_API/actions/runs/39459275998) |
| 2026-10-19 20:10 UTC | dev | — | `30d9bb5` | [run](https://github.com/mairie360/Core_API/actions/runs/39315745705) |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
