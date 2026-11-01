# Journal des releases — Core_API

<!-- release-journal-state {"dev": "564bfe60c57d50ce873255ad7561bb4fffc9f599", "prod": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "prod_tag": "v2.2.0", "staging": "cea777dde46d3a8bf89d22c64897bde045a71b9e", "updated": {"dev": "2026-11-01 20:48 UTC", "prod": "2026-10-21 09:13 UTC", "staging": "2026-10-21 03:14 UTC"}} -->
> Tenu à jour par la CI (`mairie360/CICD`, action `release/release-journal`) à chaque passage en dev, staging et prod. Ne pas modifier à la main : le fichier est réécrit au passage suivant.

| Environnement | Version | Commit | Mis à jour |
| --- | --- | --- | --- |
| prod | v2.2.0 | `cea777d` | 2026-10-21 09:13 UTC |
| staging | — | `cea777d` | 2026-10-21 03:14 UTC |
| dev | — | `564bfe6` | 2026-11-01 20:48 UTC |

## Ajouté sur main depuis la dernière release prod (v2.2.0)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : déconnecte une session à partir du seul refresh token, sans JWT, donc aussi après expiration du JWT. Révoque la session en base (`revoked_at`) et la publie sur la liste de révocation Redis. Répond `204` dans tous les cas (token inconnu, déjà révoqué ou valide), `400` si le corps est invalide, `429` au-delà de 60 par minute et par adresse, `500` si la base échoue et `503` si la révocation n'a pas pu être publiée sur Redis (à réessayer). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif d'un compte (art. 17 RGPD) via `anonymize_user()`. Les groupes, projets et événements passent à un administrateur actif, les sessions sont révoquées, le compte Keycloak est désactivé puis supprimé. Répond `404` pour un utilisateur inconnu, `409` pour l'administrateur initial ou s'il n'existe aucun autre administrateur actif, et `already_anonymized: true` si le compte est déjà effacé. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` (admin) : export des données personnelles de l'utilisateur (art. 15 et 20 RGPD), tel que renvoyé par `export_user_data()`, identifiants exclus. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `DIRECTORY_GUEST_ACCESS=none` ferme l'annuaire aux agents qui n'ont que le rôle Guest : `GET /api/v1/user/` et la fiche d'un autre utilisateur répondent `403`, et ils lisent toujours leur propre fiche. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` renvoie désormais le numéro de téléphone à tout agent, alors qu'il était masqué aux autres utilisateurs (décision de la mairie inscrite dans `access-matrix.yaml`). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Corrections
- Déconnexion avec un JWT expiré (MAIR-692) : avant, `POST /api/v1/sessions/revoke` et `POST /api/v1/sessions/logout` répondaient `401` avec un JWT expiré, et le refresh token restait valide 30 jours côté serveur. Maintenant, `POST /api/v1/auth/logout` révoque la session à partir du refresh token. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Journaux (MAIR-290) : avant, la requête journalisée contenait la query string (un e-mail recherché) et les erreurs contenaient leur valeur (token de première connexion, e-mail dans les erreurs de désérialisation), et deux journaux Keycloak nommaient le compte par son e-mail. Maintenant, la query string et les valeurs sont masquées dans les journaux et les traces, et les journaux Keycloak ne citent plus l'e-mail. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- Image d'exécution passée en distroless Debian 13, builder et image de développement en trixie, avec un test de garde `tests/dockerfile_test.rs` (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)).
- Matrice d'accès `access-matrix.yaml` vérifiée depuis l'OpenAPI, tests d'authentification RGPD et de blocage des comptes archivés, tests de limitation de débit relayés par le BFF, sans changement de code Core (MAIR-288 [#274](https://github.com/mairie360/Core_API/pull/274), MAIR-287 [#268](https://github.com/mairie360/Core_API/pull/268), MAIR-679 [#342](https://github.com/mairie360/Core_API/pull/342)).
- Simulation d'usage RGPD, test du marqueur, checklist RGPD des PR et base de test `dev-76437e3` (MAIR-497 [#273](https://github.com/mairie360/Core_API/pull/273), MAIR-290 [#265](https://github.com/mairie360/Core_API/pull/265), MAIR-295 [#272](https://github.com/mairie360/Core_API/pull/272), MAIR-289 [#277](https://github.com/mairie360/Core_API/pull/277)).
- Mises à jour : digest de `rust:1.99-bookworm` (#328, #332, #334), tokio 1.54.1 (#333), `mairie360_api_lib` (#265), CICD v4.4.1 à v4.4.3 (#325, #327, #330), exception RUSTSEC-2026-0387 (#265), entrée `.gitleaksignore` pour l'exemple du logout (#344).

## Différence dev → staging (en dev, pas encore en staging)

#### Nouveautés
- `POST /api/v1/auth/logout` (public) : déconnecte une session à partir du seul refresh token, sans JWT, donc aussi après expiration du JWT. Révoque la session en base (`revoked_at`) et la publie sur la liste de révocation Redis. Répond `204` dans tous les cas (token inconnu, déjà révoqué ou valide), `400` si le corps est invalide, `429` au-delà de 60 par minute et par adresse, `500` si la base échoue et `503` si la révocation n'a pas pu être publiée sur Redis (à réessayer). (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- `POST /api/v1/admin/users/{userId}/erase` (admin) : effacement définitif d'un compte (art. 17 RGPD) via `anonymize_user()`. Les groupes, projets et événements passent à un administrateur actif, les sessions sont révoquées, le compte Keycloak est désactivé puis supprimé. Répond `404` pour un utilisateur inconnu, `409` pour l'administrateur initial ou s'il n'existe aucun autre administrateur actif, et `already_anonymized: true` si le compte est déjà effacé. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `GET /api/v1/user/me/export` et `GET /api/v1/admin/users/{userId}/export` (admin) : export des données personnelles de l'utilisateur (art. 15 et 20 RGPD), tel que renvoyé par `export_user_data()`, identifiants exclus. (MAIR-289, [#277](https://github.com/mairie360/Core_API/pull/277))
- `DIRECTORY_GUEST_ACCESS=none` ferme l'annuaire aux agents qui n'ont que le rôle Guest : `GET /api/v1/user/` et la fiche d'un autre utilisateur répondent `403`, et ils lisent toujours leur propre fiche. Par défaut (`all`), rien ne change. (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Changements de comportement
- `GET /api/v1/user/{id}/` renvoie désormais le numéro de téléphone à tout agent, alors qu'il était masqué aux autres utilisateurs (décision de la mairie inscrite dans `access-matrix.yaml`). (MAIR-288, [#274](https://github.com/mairie360/Core_API/pull/274))

#### Corrections
- Déconnexion avec un JWT expiré (MAIR-692) : avant, `POST /api/v1/sessions/revoke` et `POST /api/v1/sessions/logout` répondaient `401` avec un JWT expiré, et le refresh token restait valide 30 jours côté serveur. Maintenant, `POST /api/v1/auth/logout` révoque la session à partir du refresh token. (MAIR-692, [#344](https://github.com/mairie360/Core_API/pull/344))
- Journaux (MAIR-290) : avant, la requête journalisée contenait la query string (un e-mail recherché) et les erreurs contenaient leur valeur (token de première connexion, e-mail dans les erreurs de désérialisation), et deux journaux Keycloak nommaient le compte par son e-mail. Maintenant, la query string et les valeurs sont masquées dans les journaux et les traces, et les journaux Keycloak ne citent plus l'e-mail. (MAIR-290, [#265](https://github.com/mairie360/Core_API/pull/265))

#### Technique
- Image d'exécution passée en distroless Debian 13, builder et image de développement en trixie, avec un test de garde `tests/dockerfile_test.rs` (MAIR-741, [#339](https://github.com/mairie360/Core_API/pull/339)).
- Matrice d'accès `access-matrix.yaml` vérifiée depuis l'OpenAPI, tests d'authentification RGPD et de blocage des comptes archivés, tests de limitation de débit relayés par le BFF, sans changement de code Core (MAIR-288 [#274](https://github.com/mairie360/Core_API/pull/274), MAIR-287 [#268](https://github.com/mairie360/Core_API/pull/268), MAIR-679 [#342](https://github.com/mairie360/Core_API/pull/342)).
- Simulation d'usage RGPD, test du marqueur, checklist RGPD des PR et base de test `dev-76437e3` (MAIR-497 [#273](https://github.com/mairie360/Core_API/pull/273), MAIR-290 [#265](https://github.com/mairie360/Core_API/pull/265), MAIR-295 [#272](https://github.com/mairie360/Core_API/pull/272), MAIR-289 [#277](https://github.com/mairie360/Core_API/pull/277)).
- Mises à jour : digest de `rust:1.99-bookworm` (#328, #332, #334), tokio 1.54.1 (#333), `mairie360_api_lib` (#265), CICD v4.4.1 à v4.4.3 (#325, #327, #330), exception RUSTSEC-2026-0387 (#265), entrée `.gitleaksignore` pour l'exemple du logout (#344).

## Différence staging → prod (en staging, pas encore en prod)

_Rien._

## Historique des passages

<!-- release-journal-history:start -->
| Date | Environnement | Version | Commit | Run |
| --- | --- | --- | --- | --- |
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
