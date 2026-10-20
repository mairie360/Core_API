# Journal des releases — Core_API

<!-- release-journal-state {"dev": "b9007977b920b85110b7e4b630bec63bf82ef365", "prod": "b14e21cf832df4d517710c5d079297098a0bbe4d", "prod_tag": "v2.1.0", "staging": "b9007977b920b85110b7e4b630bec63bf82ef365", "updated": {"dev": "2026-10-20 16:57 UTC", "staging": "2026-10-20 17:07 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.1.0 | `b14e21c` | — |
| staging | — | `b900797` | 2026-10-20 17:07 UTC |
| dev | — | `b900797` | 2026-10-20 16:57 UTC |

## Ajouté sur main depuis la dernière release prod (v2.1.0)

#### Technique
- `mairie360_api_lib` passe de 3.4.0 à 4.2.0 (la lib 4.x n'écrit plus dans Redis sans TTL, MAIR-499). Les tests d'intégration et les stacks `docker-compose*.yml` connectent désormais `core` avec le rôle Postgres `core_api` au lieu du superuser, comme en staging : une requête hors des grants échoue dès les tests avec `permission denied`. Les avis `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` et `RUSTSEC-2026-0369` (crate `zip`, utilisée seulement au build de `utoipa-swagger-ui`) sont ignorés dans l'audit. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 6 mises à jour de dépendances et de CI : `mairie360/CICD` de v4.2.1 à v4.3.2, qui ajoute l'étape de journal des releases (MAIR-571), `tokio` 1.54.0 et `utoipa-swagger-ui` 10.0.2. ([#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#314](https://github.com/mairie360/Core_API/pull/314), [#318](https://github.com/mairie360/Core_API/pull/318), [#316](https://github.com/mairie360/Core_API/pull/316), [#319](https://github.com/mairie360/Core_API/pull/319))

## Différence dev → staging (en dev, pas encore en staging)

_Rien._

## Différence staging → prod (en staging, pas encore en prod)

#### Technique
- `mairie360_api_lib` passe de 3.4.0 à 4.2.0 (la lib 4.x n'écrit plus dans Redis sans TTL, MAIR-499). Les tests d'intégration et les stacks `docker-compose*.yml` connectent désormais `core` avec le rôle Postgres `core_api` au lieu du superuser, comme en staging : une requête hors des grants échoue dès les tests avec `permission denied`. Les avis `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` et `RUSTSEC-2026-0369` (crate `zip`, utilisée seulement au build de `utoipa-swagger-ui`) sont ignorés dans l'audit. (MAIR-556, [#299](https://github.com/mairie360/Core_API/pull/299))
- 6 mises à jour de dépendances et de CI : `mairie360/CICD` de v4.2.1 à v4.3.2, qui ajoute l'étape de journal des releases (MAIR-571), `tokio` 1.54.0 et `utoipa-swagger-ui` 10.0.2. ([#310](https://github.com/mairie360/Core_API/pull/310), [#315](https://github.com/mairie360/Core_API/pull/315), [#314](https://github.com/mairie360/Core_API/pull/314), [#318](https://github.com/mairie360/Core_API/pull/318), [#316](https://github.com/mairie360/Core_API/pull/316), [#319](https://github.com/mairie360/Core_API/pull/319))

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-20 17:07 UTC | staging | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:57 UTC | dev | — | `b900797` | [run](https://github.com/mairie360/Core_API/actions/runs/39459405629) |
| 2026-10-20 16:49 UTC | dev | — | `acf2207` | [run](https://github.com/mairie360/Core_API/actions/runs/39459275998) |
| 2026-10-19 20:10 UTC | dev | — | `30d9bb5` | [run](https://github.com/mairie360/Core_API/actions/runs/39315745705) |
| 2026-10-19 18:43 UTC | dev | — | `70d1f83` | [run](https://github.com/mairie360/Core_API/actions/runs/39301376946) |
<!-- release-journal-history:end -->
