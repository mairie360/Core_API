# Journal des releases — Core_API

<!-- release-journal-state {"dev": "2ba84c16995294b49d607e8445b9d3a724c32771", "prod": "a7458a5d0f5a970bae27e3a97cc01ce4657d5247", "prod_tag": "v2.3.0", "staging": "a7458a5d0f5a970bae27e3a97cc01ce4657d5247", "updated": {"dev": "2026-11-17 01:34 UTC", "prod": "2026-11-12 23:53 UTC", "staging": "2026-11-12 22:33 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.3.0 | `a7458a5` | 2026-11-12 23:53 UTC |
| staging | — | `a7458a5` | 2026-11-12 22:33 UTC |
| dev | — | `2ba84c1` | 2026-11-17 01:34 UTC |

## Ajouté sur main depuis la dernière release prod (v2.3.0)

- test: MAIR-1002 Speed up the APIs' CI tests (Rust cache, one database per test, optimized dependencies) ([#401](https://github.com/mairie360/Core_API/pull/401))
- chore(deps): update rust crate mairie360_api_lib to v4.4.0 (#395) ([#395](https://github.com/mairie360/Core_API/pull/395))
- fix: MAIR-247 Translate the APIs' OpenAPI descriptions and messages into English (#391) ([#391](https://github.com/mairie360/Core_API/pull/391))
- docs: MAIR-246 Update the stale CLAUDE.md files (#389) ([#389](https://github.com/mairie360/Core_API/pull/389))
- chore(deps): MAIR-243 Bump quinn-udp to the patched 0.5.16 (RUSTSEC-2026-0538) (#393) ([#393](https://github.com/mairie360/Core_API/pull/393))
- docs: MAIR-245 Rewrite the APIs' README and API.md (#372) ([#372](https://github.com/mairie360/Core_API/pull/372))
- chore: MAIR-243 Align the test Database version on the 3.4.0 release (#383) ([#383](https://github.com/mairie360/Core_API/pull/383))
- test: MAIR-238 Test the handlers and access controls of the APIs (#378) ([#378](https://github.com/mairie360/Core_API/pull/378))
- chore(deps): update rust crate uuid to v1.29.2 (#370) ([#370](https://github.com/mairie360/Core_API/pull/370))

## Différence dev → staging (en dev, pas encore en staging)

- test: MAIR-1002 Speed up the APIs' CI tests (Rust cache, one database per test, optimized dependencies) ([#401](https://github.com/mairie360/Core_API/pull/401))
- chore(deps): update rust crate mairie360_api_lib to v4.4.0 (#395) ([#395](https://github.com/mairie360/Core_API/pull/395))
- fix: MAIR-247 Translate the APIs' OpenAPI descriptions and messages into English (#391) ([#391](https://github.com/mairie360/Core_API/pull/391))
- docs: MAIR-246 Update the stale CLAUDE.md files (#389) ([#389](https://github.com/mairie360/Core_API/pull/389))
- chore(deps): MAIR-243 Bump quinn-udp to the patched 0.5.16 (RUSTSEC-2026-0538) (#393) ([#393](https://github.com/mairie360/Core_API/pull/393))
- docs: MAIR-245 Rewrite the APIs' README and API.md (#372) ([#372](https://github.com/mairie360/Core_API/pull/372))
- chore: MAIR-243 Align the test Database version on the 3.4.0 release (#383) ([#383](https://github.com/mairie360/Core_API/pull/383))
- test: MAIR-238 Test the handlers and access controls of the APIs (#378) ([#378](https://github.com/mairie360/Core_API/pull/378))
- chore(deps): update rust crate uuid to v1.29.2 (#370) ([#370](https://github.com/mairie360/Core_API/pull/370))

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

> Résumé Claude indisponible pour ce passage : listes brutes des commits.

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
| 2026-11-17 01:34 UTC | dev | — | `2ba84c1` | [run](https://github.com/mairie360/Core_API/actions/runs/43992420227) |
| 2026-11-16 23:11 UTC | dev | — | `ca11d24` | [run](https://github.com/mairie360/Core_API/actions/runs/43974508602) |
| 2026-11-16 17:46 UTC | dev | — | `a772ae3` | [run](https://github.com/mairie360/Core_API/actions/runs/43919013944) |
| 2026-11-16 17:29 UTC | dev | — | `611f4c9` | [run](https://github.com/mairie360/Core_API/actions/runs/43916502710) |
| 2026-11-16 16:56 UTC | dev | — | `39868da` | [run](https://github.com/mairie360/Core_API/actions/runs/43909682505) |
| 2026-11-16 16:38 UTC | dev | — | `baf2bd0` | [run](https://github.com/mairie360/Core_API/actions/runs/43905315143) |
| 2026-11-13 01:07 UTC | dev | — | `3c43cc5` | [run](https://github.com/mairie360/Core_API/actions/runs/43340075089) |
| 2026-11-12 23:53 UTC | prod | v2.3.0 | `a7458a5` | [run](https://github.com/mairie360/Core_API/actions/runs/43330527369) |
| 2026-11-12 22:33 UTC | staging | — | `a7458a5` | [run](https://github.com/mairie360/Core_API/actions/runs/43319667465) |
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
