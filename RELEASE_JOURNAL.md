# Journal des releases — Core_API

<!-- release-journal-state {"dev": "0d723d49c49a3531b34ffce045720a16e8f538b1", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-10-21 09:18 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `0d723d4` | 2026-10-21 09:18 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Technique
- Pin CICD relevé de v4.4.0 à v4.4.3 dans `cicd.yml`, `promote-staging.yml` et `promote-prod.yml` (MAIR-612, [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330)) : d'après les descriptions de PR, ces mises à jour corrigent la chaîne de release interne (calcul de version, création du tag et de la release). Aucun changement de code de l'API.
- Image de base `rust:1.99-bookworm` figée sur un nouveau digest dans `development.Dockerfile` (MAIR-427, [#328](https://github.com/mairie360/Core_API/pull/328), sans clé Jira) : même toolchain que la production, sans effet pour les utilisateurs.

## Différence dev → staging (en dev, pas encore en staging)

#### Technique
- Pin CICD relevé de v4.4.0 à v4.4.3 dans `cicd.yml`, `promote-staging.yml` et `promote-prod.yml` (MAIR-612, [#325](https://github.com/mairie360/Core_API/pull/325), [#327](https://github.com/mairie360/Core_API/pull/327), [#330](https://github.com/mairie360/Core_API/pull/330)) : d'après les descriptions de PR, ces mises à jour corrigent la chaîne de release interne (calcul de version, création du tag et de la release). Aucun changement de code de l'API.
- Image de base `rust:1.99-bookworm` figée sur un nouveau digest dans `development.Dockerfile` (MAIR-427, [#328](https://github.com/mairie360/Core_API/pull/328), sans clé Jira) : même toolchain que la production, sans effet pour les utilisateurs.

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-10-21 09:18 UTC | dev | — | `0d723d4` | [run](https://github.com/mairie360/Core_API/actions/runs/39574704778) |
| 2026-10-21 09:13 UTC | prod | v2.2.0 | `cea777d` | [run](https://github.com/mairie360/Core_API/actions/runs/39574731861) |
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
