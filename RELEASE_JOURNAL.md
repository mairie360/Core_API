# Journal des releases — Core_API

<!-- release-journal-state {"dev": "b9007977b920b85110b7e4b630bec63bf82ef365", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "b14e21cf832df4d517710c5d079297098a0bbe4d", "updated": {"dev": "2026-10-20 16:57 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `b14e21c` | — |
| dev | — | `b900797` | 2026-10-20 16:57 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Technique
- Les tests et les stacks `docker-compose*.yml` connectent désormais Core avec le rôle Postgres `core_api`, comme en staging, au lieu du superuser. Une requête non couverte par les grants échoue donc avec `permission denied`. `mairie360_api_lib` passe de 3.4.0 à 4.2.0 : d'après la PR, la 4.x n'écrit plus dans Redis sans TTL. Les avis RUSTSEC-2026-0367/0368/0369 (zip, dépendance de build de `utoipa-swagger-ui`) sont ignorés dans `cargo audit`. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 6 mises à jour de dépendances et de CI : workflow `mairie360/CICD` de v4.2.1 à v4.3.2, qui ajoute l'étape de journal des releases (MAIR-571, [#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#314](https://github.com/mairie360/Core_API/pull/314), [#318](https://github.com/mairie360/Core_API/pull/318)), `tokio` 1.54.0 ([#316](https://github.com/mairie360/Core_API/pull/316)) et `utoipa-swagger-ui` 10.0.2 ([#319](https://github.com/mairie360/Core_API/pull/319)).

## Différence dev → staging (en dev, pas encore en staging)

#### Technique
- Les tests et les stacks `docker-compose*.yml` connectent désormais Core avec le rôle Postgres `core_api`, comme en staging, au lieu du superuser. Une requête non couverte par les grants échoue donc avec `permission denied`. `mairie360_api_lib` passe de 3.4.0 à 4.2.0 : d'après la PR, la 4.x n'écrit plus dans Redis sans TTL. Les avis RUSTSEC-2026-0367/0368/0369 (zip, dépendance de build de `utoipa-swagger-ui`) sont ignorés dans `cargo audit`. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 6 mises à jour de dépendances et de CI : workflow `mairie360/CICD` de v4.2.1 à v4.3.2, qui ajoute l'étape de journal des releases (MAIR-571, [#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#314](https://github.com/mairie360/Core_API/pull/314), [#318](https://github.com/mairie360/Core_API/pull/318)), `tokio` 1.54.0 ([#316](https://github.com/mairie360/Core_API/pull/316)) et `utoipa-swagger-ui` 10.0.2 ([#319](https://github.com/mairie360/Core_API/pull/319)).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-20 16:57 UTC | dev | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:49 UTC | dev | — | `acf2207` | [run](https://github.com/mairie360/Core_API/actions/runs/39459275998) |
| 2026-10-19 20:10 UTC | dev | — | `30d9bb5` | [run](https://github.com/mairie360/Core_API/actions/runs/39315745705) |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
