# Journal des releases — Core_API

<!-- release-journal-state {"dev": "30d9bb58589fd0cf633d38ac6fd9b780584f06c9", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "b14e21cf832df4d517710c5d079297098a0bbe4d", "updated": {"dev": "2026-10-19 20:10 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `b14e21c` | — |
| dev | — | `30d9bb5` | 2026-10-19 20:10 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Changements de comportement
- La bibliothèque `mairie360_api_lib` utilisée par l'API passe de 3.4.0 à 4.2.0. D'après la PR, la lib 4.x n'écrit plus dans Redis de clé sans TTL (MAIR-499). Le diff étant tronqué, l'effet exact à l'exécution reste à vérifier. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))

#### Technique
- Les tests d'intégration et les stacks `docker-compose*.yml` connectent désormais Core avec le rôle Postgres `core_api` au lieu du superuser, comme en staging : `DB_USER: core_api` et `-Dcore_api_password` passé à Liquibase. Postgres, Liquibase et le seeder gardent le superuser. Un test de régression vérifie que `core_api` se voit refuser `projects` (`42501`). Trois avis `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` et `RUSTSEC-2026-0369` sont ignorés dans `.cargo/audit.toml` : ils concernent `zip`, utilisé uniquement au build de `utoipa-swagger-ui`. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 4 mises à jour de dépendances :
  - workflow `mairie360/CICD` de v4.2.1 à v4.3.1, qui ajoute l'étape « Release journal » (MAIR-571, [#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#314](https://github.com/mairie360/Core_API/pull/314)) ;
  - crate `tokio` de 1.53.3 à 1.54.0 ([#316](https://github.com/mairie360/Core_API/pull/316)).

## Différence dev → staging (en dev, pas encore en staging)

#### Changements de comportement
- La bibliothèque `mairie360_api_lib` utilisée par l'API passe de 3.4.0 à 4.2.0. D'après la PR, la lib 4.x n'écrit plus dans Redis de clé sans TTL (MAIR-499). Le diff étant tronqué, l'effet exact à l'exécution reste à vérifier. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))

#### Technique
- Les tests d'intégration et les stacks `docker-compose*.yml` connectent désormais Core avec le rôle Postgres `core_api` au lieu du superuser, comme en staging : `DB_USER: core_api` et `-Dcore_api_password` passé à Liquibase. Postgres, Liquibase et le seeder gardent le superuser. Un test de régression vérifie que `core_api` se voit refuser `projects` (`42501`). Trois avis `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` et `RUSTSEC-2026-0369` sont ignorés dans `.cargo/audit.toml` : ils concernent `zip`, utilisé uniquement au build de `utoipa-swagger-ui`. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 4 mises à jour de dépendances :
  - workflow `mairie360/CICD` de v4.2.1 à v4.3.1, qui ajoute l'étape « Release journal » (MAIR-571, [#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#314](https://github.com/mairie360/Core_API/pull/314)) ;
  - crate `tokio` de 1.53.3 à 1.54.0 ([#316](https://github.com/mairie360/Core_API/pull/316)).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-19 20:10 UTC | dev | — | `30d9bb5` | [run](https://github.com/mairie360/Core_API/actions/runs/39315745705) |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
