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
use mairie360_api_lib::test_setup::queries_setup::seed_password_hash;

async fn register_fresh_user(pool: &SmartDatabase) -> u64 {
    let email = format!("preferences_{}@example.com", uuid::Uuid::new_v4());
    let _: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "Prefs",
            "User",
            &email,
            seed_password_hash(),
            None,
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
async fn get_preferences_without_row_is_all_null() {
    let pool = get_pool().await;
    let user_id = register_fresh_user(&pool).await;

    let preferences: UserPreferences = pool
        .fetch_one(&GetPreferencesQueryView::new(user_id))
        .await
        .unwrap();

    // Every display preference is the default, and the welcome tour is not done (never `null`).
    assert_eq!(
        preferences,
        UserPreferences {
            onboarding_completed: Some(false),
            ..UserPreferences::default()
        }
    );
}

#[tokio::test]
async fn patch_preferences_upserts_and_keeps_absent_fields() {
    let pool = get_pool().await;
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
async fn patch_preferences_with_an_unknown_theme_is_refused_by_the_database() {
    let pool = get_pool().await;
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
async fn notification_settings_round_trip() {
    let pool = get_pool().await;
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

/// MAIR-823: auto-save, system updates and login alerts are stored like the other settings.
#[tokio::test]
async fn mair_823_settings_v2_fields_round_trip() {
    let pool = get_pool().await;
    let user_id = register_fresh_user(&pool).await;

    let created = patch_preferences(
        &pool,
        user_id,
        &PreferencesPatch {
            autosave: Some(Some(true)),
            ..PreferencesPatch::default()
        },
    )
    .await;
    assert_eq!(created.autosave, Some(true));
    let reset = patch_preferences(
        &pool,
        user_id,
        &PreferencesPatch {
            autosave: Some(None),
            ..PreferencesPatch::default()
        },
    )
    .await;
    assert_eq!(reset.autosave, None);

    let rows: Vec<UserNotificationSettings> = pool
        .fetch_all(&PatchNotificationSettingsQueryView::new(
            user_id,
            NotificationSettingsPatch {
                system_updates: Some(Some(false)),
                login_alerts: Some(Some(true)),
                ..NotificationSettingsPatch::default()
            },
        ))
        .await
        .unwrap();
    assert_eq!(rows[0].system_updates, Some(false));
    assert_eq!(rows[0].login_alerts, Some(true));
    let read: UserNotificationSettings = pool
        .fetch_one(&GetNotificationSettingsQueryView::new(user_id))
        .await
        .unwrap();
    assert_eq!(read.login_alerts, Some(true));
    assert_eq!(read.email, None);
}
