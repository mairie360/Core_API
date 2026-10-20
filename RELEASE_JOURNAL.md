# Journal des releases — Core_API

<!-- release-journal-state {"dev": "acf2207b124f75e21191febbd4d44a58952ba9ca", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "b14e21cf832df4d517710c5d079297098a0bbe4d", "updated": {"dev": "2026-10-20 16:49 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `b14e21c` | — |
| dev | — | `acf2207` | 2026-10-20 16:49 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Technique
- Les tests d'intégration et les stacks `docker-compose*.yml` connectent désormais Core à Postgres avec le rôle `core_api`, comme en staging, et non plus avec le superuser. Postgres, Liquibase et le seeder gardent le superuser, et Liquibase reçoit `-Dcore_api_password`. Une requête non couverte par les grants échoue maintenant dès les tests avec `permission denied`. Le test de régression `tests/queries/db_role.rs` est ajouté. `mairie360_api_lib` passe de 3.4.0 à 4.2.0 ; d'après la PR, la lib 4.x n'écrit plus dans Redis sans TTL (MAIR-499). Les avis `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` et `RUSTSEC-2026-0369` sont ignorés : ils concernent `zip`, dépendance de build de `utoipa-swagger-ui`. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 5 mises à jour de dépendances : le workflow `mairie360/CICD` passe de v4.2.1 à v4.3.2 en quatre étapes et ajoute l'étape « Release journal » (MAIR-571) ; `tokio` passe de 1.53.3 à 1.54.0. ([#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#314](https://github.com/mairie360/Core_API/pull/314), [#318](https://github.com/mairie360/Core_API/pull/318), [#316](https://github.com/mairie360/Core_API/pull/316))

## Différence dev → staging (en dev, pas encore en staging)

#### Technique
- Les tests d'intégration et les stacks `docker-compose*.yml` connectent désormais Core à Postgres avec le rôle `core_api`, comme en staging, et non plus avec le superuser. Postgres, Liquibase et le seeder gardent le superuser, et Liquibase reçoit `-Dcore_api_password`. Une requête non couverte par les grants échoue maintenant dès les tests avec `permission denied`. Le test de régression `tests/queries/db_role.rs` est ajouté. `mairie360_api_lib` passe de 3.4.0 à 4.2.0 ; d'après la PR, la lib 4.x n'écrit plus dans Redis sans TTL (MAIR-499). Les avis `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` et `RUSTSEC-2026-0369` sont ignorés : ils concernent `zip`, dépendance de build de `utoipa-swagger-ui`. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 5 mises à jour de dépendances : le workflow `mairie360/CICD` passe de v4.2.1 à v4.3.2 en quatre étapes et ajoute l'étape « Release journal » (MAIR-571) ; `tokio` passe de 1.53.3 à 1.54.0. ([#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#314](https://github.com/mairie360/Core_API/pull/314), [#318](https://github.com/mairie360/Core_API/pull/318), [#316](https://github.com/mairie360/Core_API/pull/316))

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-20 16:49 UTC | dev | — | `acf2207` | [run](https://github.com/mairie360/Core_API/actions/runs/39459275998) |
| 2026-10-19 20:10 UTC | dev | — | `30d9bb5` | [run](https://github.com/mairie360/Core_API/actions/runs/39315745705) |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
