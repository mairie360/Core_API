# Journal des releases — Core_API

<!-- release-journal-state {"dev": "a7458a5d0f5a970bae27e3a97cc01ce4657d5247", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-12 22:24 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `a7458a5` | 2026-11-12 22:24 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

- fix: MAIR-1001 Release staging: align internal versions and deploy ([#381](https://github.com/mairie360/Core_API/pull/381))
- fix: MAIR-1001 Release staging: align internal versions and deploy ([#379](https://github.com/mairie360/Core_API/pull/379))
- fix: MAIR-262 Integration tests - API functional tests, BFF business tests and regression tests (#375) ([#375](https://github.com/mairie360/Core_API/pull/375))
- chore(deps): update gcr.io/distroless/cc-debian13:nonroot docker digest to cc66d22 (#340) ([#340](https://github.com/mairie360/Core_API/pull/340))
- fix(deps): MAIR-740 APIs: fix the cargo audit RustSec alerts (actix-http, libsqlite3-sys) (#374) ([#374](https://github.com/mairie360/Core_API/pull/374))
- chore(deps): update rust:1.99-trixie docker digest to 8a4e90c (#369) ([#369](https://github.com/mairie360/Core_API/pull/369))
- chore(deps): update rust:1.99-trixie docker digest to bc2c4c2 (#367) ([#367](https://github.com/mairie360/Core_API/pull/367))
- feat: MAIR-609 Core/BFF_user - Accès temporaire à la console Keycloak pour les admins de mairie (#364) ([#364](https://github.com/mairie360/Core_API/pull/364))
- chore: MAIR-609 Core/BFF_user - Accès temporaire à la console Keycloak pour les admins de mairie (#365) ([#365](https://github.com/mairie360/Core_API/pull/365))
- chore(deps): update rust:1.99-trixie docker digest to 4c4c8ec (#363) ([#363](https://github.com/mairie360/Core_API/pull/363))
- chore(deps): update rust dependencies (#335) ([#335](https://github.com/mairie360/Core_API/pull/335))
- chore(deps): pin dependencies (#350) ([#350](https://github.com/mairie360/Core_API/pull/350))
- fix: MAIR-746 [Audit 30/10] BFF_Settings / Core : vérification du mot de passe sans limiteur via PATCH /settings/profile (N4) (#359) ([#359](https://github.com/mairie360/Core_API/pull/359))
- ci: MAIR-771 APIs, BFFs, Database : monter CICD en v5.2.2 pour activer les contrôles RGPD (#357) ([#357](https://github.com/mairie360/Core_API/pull/357))
- fix: MAIR-678 [Audit 23/10] Core_API : empêcher de rétrograder ou retirer le dernier administrateur (A5) (#355) ([#355](https://github.com/mairie360/Core_API/pull/355))
- fix: MAIR-767 Core API - Renvoyer les dates de session au format ISO 8601 ([#354](https://github.com/mairie360/Core_API/pull/354))
- fix: MAIR-678 [Audit 23/10] Core_API : empêcher de rétrograder ou retirer le dernier administrateur (A5) (#352) ([#352](https://github.com/mairie360/Core_API/pull/352))
- fix: MAIR-692 [Audit 23/10] Core_API : la déconnexion échoue avec un JWT expiré, le refresh token n'est pas révoqué (E4) (#349) ([#349](https://github.com/mairie360/Core_API/pull/349))
- feat: MAIR-680 [Audit 23/10] Toutes les APIs et BFFs : remplacer le secret JWT HS256 partagé par 12 services (A1) (#346) ([#346](https://github.com/mairie360/Core_API/pull/346))
- fix: MAIR-692 [Audit 23/10] Core_API : la déconnexion échoue avec un JWT expiré, le refresh token n'est pas révoqué (E4) (#344) ([#344](https://github.com/mairie360/Core_API/pull/344))
- test: MAIR-679 [Audit 23/10] BFF_user / bffs_lib / Core : « mot de passe oublié » verrouillable pour toute l'instance (A6) (#342) ([#342](https://github.com/mairie360/Core_API/pull/342))
- fix: MAIR-741 APIs : passer l'image d'exécution en distroless Debian 13 (CVE-2026-84782) (#339) ([#339](https://github.com/mairie360/Core_API/pull/339))
- feat: MAIR-497 Simulation d'usage RGPD et revue IA des logs et de la base avant la prod (#273) ([#273](https://github.com/mairie360/Core_API/pull/273))
- feat: MAIR-289 Anonymiser et exporter les données d'un utilisateur, avec tests d'effacement (#277) ([#277](https://github.com/mairie360/Core_API/pull/277))
- feat: MAIR-288 Définir la matrice d'accès des APIs et la vérifier depuis l'OpenAPI (#274) ([#274](https://github.com/mairie360/Core_API/pull/274))
- test: MAIR-287 Tester l'authentification de Core_API contre les exigences RGPD et bloquer les comptes archivés (#268) ([#268](https://github.com/mairie360/Core_API/pull/268))
- fix: MAIR-290 Détecter les données personnelles dans les journaux avec un test du marqueur (#265) ([#265](https://github.com/mairie360/Core_API/pull/265))
- ci: MAIR-295 Ajouter une checklist RGPD aux PR et des rappels d'échéances automatiques (#272) ([#272](https://github.com/mairie360/Core_API/pull/272))
- chore(deps): update rust:1.99-bookworm docker digest to 48b7f43 (#334) ([#334](https://github.com/mairie360/Core_API/pull/334))
- chore(deps): update rust:1.99-bookworm docker digest to edaa2a8 (#332) ([#332](https://github.com/mairie360/Core_API/pull/332))
- chore(deps): update rust crate tokio to v1.54.1 (#333) ([#333](https://github.com/mairie360/Core_API/pull/333))
- ci: MAIR-612 Release prod: align internal versions and deploy (#330) ([#330](https://github.com/mairie360/Core_API/pull/330))
- chore(deps): update rust:1.99-bookworm docker digest to 5c1a173 (#328) ([#328](https://github.com/mairie360/Core_API/pull/328))
- ci: MAIR-612 Release prod: align internal versions and deploy (#327) ([#327](https://github.com/mairie360/Core_API/pull/327))
- ci: MAIR-612 Release prod: align internal versions and deploy (#325) ([#325](https://github.com/mairie360/Core_API/pull/325))

## Différence dev → staging (en dev, pas encore en staging)

- fix: MAIR-1001 Release staging: align internal versions and deploy ([#381](https://github.com/mairie360/Core_API/pull/381))
- fix: MAIR-1001 Release staging: align internal versions and deploy ([#379](https://github.com/mairie360/Core_API/pull/379))
- fix: MAIR-262 Integration tests - API functional tests, BFF business tests and regression tests (#375) ([#375](https://github.com/mairie360/Core_API/pull/375))
- chore(deps): update gcr.io/distroless/cc-debian13:nonroot docker digest to cc66d22 (#340) ([#340](https://github.com/mairie360/Core_API/pull/340))
- fix(deps): MAIR-740 APIs: fix the cargo audit RustSec alerts (actix-http, libsqlite3-sys) (#374) ([#374](https://github.com/mairie360/Core_API/pull/374))
- chore(deps): update rust:1.99-trixie docker digest to 8a4e90c (#369) ([#369](https://github.com/mairie360/Core_API/pull/369))
- chore(deps): update rust:1.99-trixie docker digest to bc2c4c2 (#367) ([#367](https://github.com/mairie360/Core_API/pull/367))
- feat: MAIR-609 Core/BFF_user - Accès temporaire à la console Keycloak pour les admins de mairie (#364) ([#364](https://github.com/mairie360/Core_API/pull/364))
- chore: MAIR-609 Core/BFF_user - Accès temporaire à la console Keycloak pour les admins de mairie (#365) ([#365](https://github.com/mairie360/Core_API/pull/365))
- chore(deps): update rust:1.99-trixie docker digest to 4c4c8ec (#363) ([#363](https://github.com/mairie360/Core_API/pull/363))
- chore(deps): update rust dependencies (#335) ([#335](https://github.com/mairie360/Core_API/pull/335))
- chore(deps): pin dependencies (#350) ([#350](https://github.com/mairie360/Core_API/pull/350))
- fix: MAIR-746 [Audit 30/10] BFF_Settings / Core : vérification du mot de passe sans limiteur via PATCH /settings/profile (N4) (#359) ([#359](https://github.com/mairie360/Core_API/pull/359))
- ci: MAIR-771 APIs, BFFs, Database : monter CICD en v5.2.2 pour activer les contrôles RGPD (#357) ([#357](https://github.com/mairie360/Core_API/pull/357))
- fix: MAIR-678 [Audit 23/10] Core_API : empêcher de rétrograder ou retirer le dernier administrateur (A5) (#355) ([#355](https://github.com/mairie360/Core_API/pull/355))
- fix: MAIR-767 Core API - Renvoyer les dates de session au format ISO 8601 ([#354](https://github.com/mairie360/Core_API/pull/354))
- fix: MAIR-678 [Audit 23/10] Core_API : empêcher de rétrograder ou retirer le dernier administrateur (A5) (#352) ([#352](https://github.com/mairie360/Core_API/pull/352))
- fix: MAIR-692 [Audit 23/10] Core_API : la déconnexion échoue avec un JWT expiré, le refresh token n'est pas révoqué (E4) (#349) ([#349](https://github.com/mairie360/Core_API/pull/349))
- feat: MAIR-680 [Audit 23/10] Toutes les APIs et BFFs : remplacer le secret JWT HS256 partagé par 12 services (A1) (#346) ([#346](https://github.com/mairie360/Core_API/pull/346))
- fix: MAIR-692 [Audit 23/10] Core_API : la déconnexion échoue avec un JWT expiré, le refresh token n'est pas révoqué (E4) (#344) ([#344](https://github.com/mairie360/Core_API/pull/344))
- test: MAIR-679 [Audit 23/10] BFF_user / bffs_lib / Core : « mot de passe oublié » verrouillable pour toute l'instance (A6) (#342) ([#342](https://github.com/mairie360/Core_API/pull/342))
- fix: MAIR-741 APIs : passer l'image d'exécution en distroless Debian 13 (CVE-2026-84782) (#339) ([#339](https://github.com/mairie360/Core_API/pull/339))
- feat: MAIR-497 Simulation d'usage RGPD et revue IA des logs et de la base avant la prod (#273) ([#273](https://github.com/mairie360/Core_API/pull/273))
- feat: MAIR-289 Anonymiser et exporter les données d'un utilisateur, avec tests d'effacement (#277) ([#277](https://github.com/mairie360/Core_API/pull/277))
- feat: MAIR-288 Définir la matrice d'accès des APIs et la vérifier depuis l'OpenAPI (#274) ([#274](https://github.com/mairie360/Core_API/pull/274))
- test: MAIR-287 Tester l'authentification de Core_API contre les exigences RGPD et bloquer les comptes archivés (#268) ([#268](https://github.com/mairie360/Core_API/pull/268))
- fix: MAIR-290 Détecter les données personnelles dans les journaux avec un test du marqueur (#265) ([#265](https://github.com/mairie360/Core_API/pull/265))
- ci: MAIR-295 Ajouter une checklist RGPD aux PR et des rappels d'échéances automatiques (#272) ([#272](https://github.com/mairie360/Core_API/pull/272))
- chore(deps): update rust:1.99-bookworm docker digest to 48b7f43 (#334) ([#334](https://github.com/mairie360/Core_API/pull/334))
- chore(deps): update rust:1.99-bookworm docker digest to edaa2a8 (#332) ([#332](https://github.com/mairie360/Core_API/pull/332))
- chore(deps): update rust crate tokio to v1.54.1 (#333) ([#333](https://github.com/mairie360/Core_API/pull/333))
- ci: MAIR-612 Release prod: align internal versions and deploy (#330) ([#330](https://github.com/mairie360/Core_API/pull/330))
- chore(deps): update rust:1.99-bookworm docker digest to 5c1a173 (#328) ([#328](https://github.com/mairie360/Core_API/pull/328))
- ci: MAIR-612 Release prod: align internal versions and deploy (#327) ([#327](https://github.com/mairie360/Core_API/pull/327))
- ci: MAIR-612 Release prod: align internal versions and deploy (#325) ([#325](https://github.com/mairie360/Core_API/pull/325))

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

> Résumé Claude indisponible pour ce passage : listes brutes des commits.

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-11-12 22:24 UTC | dev | — | `a7458a5` | [run](https://github.com/mairie360/Core_API/actions/runs/43317856707) |
| 2026-11-12 21:36 UTC | dev | — | `e5abbf8` | [run](https://github.com/mairie360/Core_API/actions/runs/43308414825) |
| 2026-11-12 13:01 UTC | dev | — | `156a9b9` | [run](https://github.com/mairie360/Core_API/actions/runs/43220637095) |
| 2026-11-12 12:02 UTC | dev | — | `b27f663` | [run](https://github.com/mairie360/Core_API/actions/runs/43212171580) |
| 2026-11-06 04:36 UTC | dev | — | `4902160` | [run](https://github.com/mairie360/Core_API/actions/runs/42162275244) |
| 2026-11-04 14:39 UTC | dev | — | `690f32b` | [run](https://github.com/mairie360/Core_API/actions/runs/41847588630) |
| 2026-11-04 14:17 UTC | dev | — | `b0a53b8` | [run](https://github.com/mairie360/Core_API/actions/runs/41843590397) |
| 2026-11-04 07:06 UTC | dev | — | `6bc9630` | [run](https://github.com/mairie360/Core_API/actions/runs/41788343275) |
| 2026-11-03 21:33 UTC | dev | — | `ca6521a` | [run](https://github.com/mairie360/Core_API/actions/runs/41719586606) |
| 2026-11-03 05:58 UTC | dev | — | `28ff46a` | [run](https://github.com/mairie360/Core_API/actions/runs/41585688519) |
| 2026-11-02 23:07 UTC | dev | — | `3b40c2a` | [run](https://github.com/mairie360/Core_API/actions/runs/41540032773) |
| 2026-11-02 23:06 UTC | dev | — | `0900864` | [run](https://github.com/mairie360/Core_API/actions/runs/41539977149) |
| 2026-11-02 17:51 UTC | dev | — | `1843364` | [run](https://github.com/mairie360/Core_API/actions/runs/41493127882) |
| 2026-11-02 17:23 UTC | dev | — | `7dfccfc` | [run](https://github.com/mairie360/Core_API/actions/runs/41488250282) |
| 2026-11-02 16:58 UTC | dev | — | `6657fe8` | [run](https://github.com/mairie360/Core_API/actions/runs/41484328354) |
| 2026-11-01 22:33 UTC | dev | — | `72dca5b` | [run](https://github.com/mairie360/Core_API/actions/runs/41356438605) |
| 2026-11-01 21:13 UTC | dev | — | `f309422` | [run](https://github.com/mairie360/Core_API/actions/runs/41349142175) |
| 2026-11-01 20:48 UTC | dev | — | `564bfe6` | [run](https://github.com/mairie360/Core_API/actions/runs/41347708018) |
| 2026-11-01 20:15 UTC | dev | — | `e40686f` | [run](https://github.com/mairie360/Core_API/actions/runs/41344959518) |
| 2026-10-31 01:52 UTC | dev | — | `f3229f1` | [run](https://github.com/mairie360/Core_API/actions/runs/41155994544) |
| 2026-10-22 22:08 UTC | dev | — | `619d1b8` | [run](https://github.com/mairie360/Core_API/actions/runs/39861535514) |
| 2026-10-22 06:11 UTC | dev | — | `0edb5c3` | [run](https://github.com/mairie360/Core_API/actions/runs/39731423596) |
| 2026-10-22 03:49 UTC | dev | — | `ea251a2` | [run](https://github.com/mairie360/Core_API/actions/runs/39717500210) |
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
