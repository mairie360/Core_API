use crate::common::get_pool;
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use core_api::database::users::get_notification_settings::{
    GetNotificationSettingsQueryView, UserNotificationSettings,
};
use core_api::database::users::get_preferences::{GetPreferencesQueryView, UserPreferences};
use core_api::database::users::patch_notification_settings::{
    NotificationSettingsPatch, PatchNotificationSettingsQueryView,
};
use core_api::database::users::patch_preferences::{PatchPreferencesQueryView, PreferencesPatch};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

async fn register_fresh_user(pool: &SmartDatabase) -> u64 {
    let email = format!("preferences_{}@example.com", uuid::Uuid::new_v4());
    let _: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "Prefs", "User", &email, "password", None,
        ))
        .await
        .unwrap();

    pool.fetch_scalar::<i32, _>(&GetUserIdQueryView::new(&email))
        .await
        .unwrap() as u64
}

async fn patch_preferences(
    pool: &SmartDatabase,
    user_id: u64,
    patch: &PreferencesPatch<'_>,
) -> UserPreferences {
    let rows: Vec<UserPreferences> = pool
        .fetch_all(&PatchPreferencesQueryView::new(user_id, patch))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    rows.into_iter().next().unwrap()
}

#[tokio::test]
#[serial]
async fn get_preferences_without_row_is_all_null() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let user_id = register_fresh_user(&pool).await;

    let preferences: UserPreferences = pool
        .fetch_one(&GetPreferencesQueryView::new(user_id))
        .await
        .unwrap();

    assert_eq!(preferences, UserPreferences::default());
}

#[tokio::test]
#[serial]
async fn patch_preferences_upserts_and_keeps_absent_fields() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let user_id = register_fresh_user(&pool).await;

    // First call creates the row.
    let created = patch_preferences(
        &pool,
        user_id,
        &PreferencesPatch {
            theme: Some(Some("dark")),
            font_size: Some(Some(16)),
            language: Some(Some("fr")),
            auto_open_notifications: Some(Some(true)),
            ..PreferencesPatch::default()
        },
    )
    .await;
    assert_eq!(created.theme.as_deref(), Some("dark"));
    assert_eq!(created.font_size, Some(16));
    assert_eq!(created.language.as_deref(), Some("fr"));
    assert_eq!(created.auto_open_notifications, Some(true));
    assert_eq!(created.timezone, None);

    // Second call: absent fields are kept, `Some(None)` resets to NULL.
    let updated = patch_preferences(
        &pool,
        user_id,
        &PreferencesPatch {
            timezone: Some(Some("Europe/Paris")),
            font_size: Some(None),
            ..PreferencesPatch::default()
        },
    )
    .await;
    assert_eq!(updated.theme.as_deref(), Some("dark"));
    assert_eq!(updated.language.as_deref(), Some("fr"));
    assert_eq!(updated.timezone.as_deref(), Some("Europe/Paris"));
    assert_eq!(updated.font_size, None);

    let read: UserPreferences = pool
        .fetch_one(&GetPreferencesQueryView::new(user_id))
        .await
        .unwrap();
    assert_eq!(read, updated);
}

#[tokio::test]
#[serial]
async fn patch_preferences_with_an_unknown_theme_is_refused_by_the_database() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let user_id = register_fresh_user(&pool).await;

    // The endpoint validates `theme` first; the CHECK constraint is the last line of defence.
    let result: Result<Vec<UserPreferences>, _> = pool
        .fetch_all(&PatchPreferencesQueryView::new(
            user_id,
            &PreferencesPatch {
                theme: Some(Some("neon")),
                ..PreferencesPatch::default()
            },
        ))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn notification_settings_round_trip() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let user_id = register_fresh_user(&pool).await;

    let empty: UserNotificationSettings = pool
        .fetch_one(&GetNotificationSettingsQueryView::new(user_id))
        .await
        .unwrap();
    assert_eq!(empty, UserNotificationSettings::default());

    let rows: Vec<UserNotificationSettings> = pool
        .fetch_all(&PatchNotificationSettingsQueryView::new(
            user_id,
            NotificationSettingsPatch {
                email: Some(Some(true)),
                push: Some(Some(false)),
                ..NotificationSettingsPatch::default()
            },
        ))
        .await
        .unwrap();
    assert_eq!(rows[0].email, Some(true));
    assert_eq!(rows[0].push, Some(false));

    let rows: Vec<UserNotificationSettings> = pool
        .fetch_all(&PatchNotificationSettingsQueryView::new(
            user_id,
            NotificationSettingsPatch {
                email: Some(None),
                calendar: Some(Some(true)),
                ..NotificationSettingsPatch::default()
            },
        ))
        .await
        .unwrap();
    let expected = UserNotificationSettings {
        email: None,
        push: Some(false),
        calendar: Some(true),
        ..UserNotificationSettings::default()
    };
    assert_eq!(rows[0], expected);

    let read: UserNotificationSettings = pool
        .fetch_one(&GetNotificationSettingsQueryView::new(user_id))
        .await
        .unwrap();
    assert_eq!(read, expected);
}
