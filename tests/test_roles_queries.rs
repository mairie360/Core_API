use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tokio::sync::OnceCell;

static COUNT: OnceCell<u64> = OnceCell::const_new();
static DELETE_ID: OnceCell<u64> = OnceCell::const_new();
static PATCH_ID: OnceCell<u64> = OnceCell::const_new();

static PATCH_MUTEX: OnceCell<tokio::sync::Mutex<()>> = OnceCell::const_new();

async fn setup_tests() {
    if COUNT.get().is_none() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;
        COUNT.set(0).unwrap();
        let _ = sqlx::query(
            "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, $2, true)",
        )
        .bind("Delete")
        .bind("Delete role")
        .execute(&pool)
        .await;
        let delete_id = sqlx::query("SELECT id FROM roles WHERE name = 'Delete'")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i32, _>(0);
        DELETE_ID.set(delete_id as u64).unwrap();
        let _ = sqlx::query(
            "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, $2, true)",
        )
        .bind("Patch")
        .bind("Patch role")
        .execute(&pool)
        .await;
        let patch_id = sqlx::query("SELECT id FROM roles WHERE name = 'Patch'")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i32, _>(0);
        PATCH_ID.set(patch_id as u64).unwrap();
        _ = PATCH_MUTEX.set(tokio::sync::Mutex::new(()));
    }
}

async fn get_pool(url: String) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&url) // On passe l'URL construite ici
        .await
        .expect("Failed to create Postgres pool")
}

#[cfg(test)]
mod queries_tests {
    use core_api::database::queries::roles::can_delete_role::{
        can_delete_role_query, CanDeleteRoleQueryView,
    };
    use core_api::database::queries::roles::change_role::{change_role_query, ChangeRoleQueryView};
    use core_api::database::queries::roles::create_role::{create_role_query, CreateRoleQueryView};
    use core_api::database::queries::roles::delete_role::{delete_role_query, DeleteRoleQueryView};
    use core_api::database::queries::roles::get_roles::{get_roles_query, GetRolesQueryView};
    use core_api::database::queries::roles::patch_role::{patch_role_query, PatchRoleQueryView};
    use mairie360_api_lib::database::errors::DatabaseError;
    use mairie360_api_lib::database::queries::QueryError;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_can_delete_role_success() {
        setup_tests().await;
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let result =
            can_delete_role_query(CanDeleteRoleQueryView::new(*DELETE_ID.get().unwrap()), pool)
                .await
                .unwrap();

        assert!(result);
    }

    #[tokio::test]
    #[serial]
    async fn test_can_delete_role_bad_id() {
        setup_tests().await;
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let result = can_delete_role_query(CanDeleteRoleQueryView::new(999), pool).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err, DatabaseError::Query(QueryError::NoResults));
    }

    //test get roles
    #[tokio::test]
    #[serial]
    async fn test_get_roles() {
        setup_tests().await;
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let roles = get_roles_query(GetRolesQueryView {}, pool).await.unwrap();

        assert!(roles.len() >= 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_create_role_success() {
        setup_tests().await;
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let nb_roles = get_roles_query(GetRolesQueryView {}, pool.clone())
            .await
            .unwrap()
            .len();

        let view = CreateRoleQueryView::new("Create", "Create role", Some(false));
        let result = create_role_query(view, pool.clone()).await;

        assert!(result.is_ok());

        let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
            .await
            .unwrap();
        assert!(roles.len() >= nb_roles);

        for role in roles {
            if role.description().is_some_and(|d| d == "Create role") {
                assert!(role.id() > 2);
                assert_eq!(role.name(), "Create");
                assert_eq!(role.description().unwrap(), "Create role");
                assert!(role.updated_at().is_some());
                assert!(!role.can_be_deleted());
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_change_role_success() {
        setup_tests().await;
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let view = CreateRoleQueryView::new("Change_test", "Change_test role", Some(true));
        let result = create_role_query(view, pool.clone()).await;

        assert!(result.is_ok());

        let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
            .await
            .unwrap();

        let mut new_role_id: i32 = 0;
        for role in roles {
            if role.name() == "Change_test" {
                new_role_id = role.id();
                break;
            }
        }

        let view = ChangeRoleQueryView::new(
            new_role_id as u64,
            "Change_Admin",
            "Change_Administrateur",
            Some(true),
        );
        let result = change_role_query(view, pool.clone()).await;

        assert!(result.is_ok());

        let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
            .await
            .unwrap();

        for role in roles {
            if role
                .description()
                .is_some_and(|d| d == "Change_Administrateur".to_string())
            {
                assert_eq!(role.id(), new_role_id);
                assert_eq!(role.name(), "Change_Admin");
                assert_eq!(role.description().unwrap(), "Change_Administrateur");
                assert!(role.updated_at().is_some());
                assert!(role.can_be_deleted());
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_change_role_bad_id() {
        setup_tests().await;
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let view = ChangeRoleQueryView::new(999, "Admin", "Administrateur", Some(false));
        let result = change_role_query(view, pool.clone()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_patch_role_name() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let view = PatchRoleQueryView::new(
            *PATCH_ID.get().unwrap(),
            Some("Patch".to_string()),
            None,
            None,
        );
        let _guard = PATCH_MUTEX.get().unwrap().lock().await;
        let result = patch_role_query(view, pool.clone()).await;

        let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
            .await
            .unwrap();

        assert!(result.is_ok());
        for role in roles {
            if role.id() == *PATCH_ID.get().unwrap() as i32 {
                assert_eq!(role.name(), "Patch");
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_patch_role_description() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let view = PatchRoleQueryView::new(
            *PATCH_ID.get().unwrap(),
            None,
            Some("Patch description".to_string()),
            None,
        );
        let _guard = PATCH_MUTEX.get().unwrap().lock().await;
        let result = patch_role_query(view, pool.clone()).await;

        let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
            .await
            .unwrap();

        assert!(result.is_ok());
        for role in roles {
            if role.id() == *PATCH_ID.get().unwrap() as i32 {
                assert_eq!(role.description().unwrap(), "Patch description");
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_patch_role_can_be_deleted_to_false() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let view = PatchRoleQueryView::new(*PATCH_ID.get().unwrap(), None, None, Some(Some(false)));
        let _guard = PATCH_MUTEX.get().unwrap().lock().await;
        let result = patch_role_query(view, pool.clone()).await;

        let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
            .await
            .unwrap();

        assert!(result.is_ok());
        for role in roles {
            if role.id() == *PATCH_ID.get().unwrap() as i32 {
                assert_eq!(role.can_be_deleted(), false);
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_role() {
        setup_tests().await;
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let view = DeleteRoleQueryView::new(*DELETE_ID.get().unwrap());
        let result = delete_role_query(view, pool.clone()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_role_bad_id() {
        setup_tests().await;
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let view = DeleteRoleQueryView::new(999);
        let result = delete_role_query(view, pool.clone()).await;

        assert!(result.is_ok());
    }
}
