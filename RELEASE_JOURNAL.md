# Journal des releases — Core_API

<!-- release-journal-state {"dev": "70d1f833314627abe701ebe5280d8e9f03e25a4c", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "b14e21cf832df4d517710c5d079297098a0bbe4d", "updated": {"dev": "2026-10-19 18:43 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `b14e21c` | — |
| dev | — | `70d1f83` | 2026-10-19 18:43 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Technique
- Les tests de Core_API exécutent désormais le code testé avec le rôle Postgres `core_api`, comme en staging, au lieu du superuser. Les fixtures gardent le superuser via `get_raw_pool(get_shared_db().1)`. `mairie360_api_lib` passe en 4.1.0 pour `get_shared_db_as`, et `tests/endpoints/revocation_list.rs` insère ses clés Redis avec `set_ex`. Un test de régression `tests/queries/db_role.rs` vérifie que `core_api` se voit refuser `projects` (`42501`). Enfin, dans `docker-compose*.yml`, `core` se connecte en `core_api`. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 3 mises à jour de dépendances : `mairie360/CICD` passe de v4.2.1 à v4.3.0 en deux étapes, et `tokio` passe de 1.53.3 à 1.54.0. ([#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#316](https://github.com/mairie360/Core_API/pull/316))

## Différence dev → staging (en dev, pas encore en staging)

#### Technique
- Les tests de Core_API exécutent désormais le code testé avec le rôle Postgres `core_api`, comme en staging, au lieu du superuser. Les fixtures gardent le superuser via `get_raw_pool(get_shared_db().1)`. `mairie360_api_lib` passe en 4.1.0 pour `get_shared_db_as`, et `tests/endpoints/revocation_list.rs` insère ses clés Redis avec `set_ex`. Un test de régression `tests/queries/db_role.rs` vérifie que `core_api` se voit refuser `projects` (`42501`). Enfin, dans `docker-compose*.yml`, `core` se connecte en `core_api`. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 3 mises à jour de dépendances : `mairie360/CICD` passe de v4.2.1 à v4.3.0 en deux étapes, et `tokio` passe de 1.53.3 à 1.54.0. ([#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#316](https://github.com/mairie360/Core_API/pull/316))

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
