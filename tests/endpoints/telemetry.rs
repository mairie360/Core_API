//! OpenTelemetry tracing (MAIR-131): each request yields a root span carrying the route and the
//! status, continues an incoming `traceparent`, and carries the SQL it ran as span events.

use actix_web::{test, web, App};
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use core_api::endpoints::{config, public_config};
use core_api::telemetry::trace_layer;
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::seed_password_hash;
use mairie360_api_lib::{
    security::JwtMiddleware, state::AppState, test_setup::queries_setup::get_shared_db,
};
use opentelemetry::global;
use opentelemetry::Value;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider, SpanData};
use serial_test::serial;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::layer::SubscriberExt;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "telemetry_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
    global::set_text_map_propagator(TraceContextPropagator::new());
});

const PREFERENCES: &str = "/api/v1/user/me/preferences/";
const PREFERENCES_ROUTE: &str = "GET /api/v1/user/me/preferences/";

/// Same mounting as `main.rs`, `TracingLogger` outermost.
macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .wrap(TracingLogger::default())
                .app_data($state.clone())
                .configure(public_config)
                .service(web::scope("/api").wrap(JwtMiddleware).configure(config)),
        )
        .await
    };
}

/// Registers a fresh user and returns a JWT for it.
async fn fresh_user_jwt(state: &web::Data<AppState>) -> String {
    let email = format!("telemetry_endpoint_{}@example.com", uuid::Uuid::new_v4());
    let db = state.get_smart_db();
    let _: bool = db
        .fetch_scalar(&RegisterUserQueryView::new(
            "Telemetry",
            "Endpoint",
            &email,
            seed_password_hash(),
            None,
        ))
        .await
        .unwrap();
    let user_id: i32 = db
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();
    generate_jwt(&user_id.to_string(), "User").unwrap()
}

fn attribute<'a>(span: &'a SpanData, key: &str) -> Option<&'a Value> {
    span.attributes
        .iter()
        .find(|kv| kv.key.as_str() == key)
        .map(|kv| &kv.value)
}

#[tokio::test]
#[serial]
async fn a_request_is_exported_as_a_span_with_its_sql() {
    std::sync::LazyLock::force(&INIT);
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let _subscriber = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(trace_layer(&provider)),
    );

    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);
    let jwt = fresh_user_jwt(&state).await;
    exporter.reset();

    let trace_id = "4bf92f3577b34da6a3ce929d0e0e4736";
    let req = test::TestRequest::get()
        .uri(PREFERENCES)
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .insert_header(("traceparent", format!("00-{trace_id}-00f067aa0ba902b7-01")))
        .to_request();
    let response = test::call_service(&app, req).await;
    assert!(response.status().is_success());
    // The root span closes with the response body: drop it before reading the exported spans.
    drop(response);

    provider.force_flush().expect("flush the spans");
    let spans = exporter.get_finished_spans().unwrap();
    let request = spans
        .iter()
        .find(|span| span.name == PREFERENCES_ROUTE)
        .unwrap_or_else(|| panic!("no span named {PREFERENCES_ROUTE}: {spans:#?}"));

    assert_eq!(
        request.span_context.trace_id().to_string(),
        trace_id,
        "the incoming traceparent must be continued"
    );
    assert_eq!(
        attribute(request, "http.status_code"),
        Some(&Value::I64(200))
    );
    assert_eq!(
        attribute(request, "http.route"),
        Some(&Value::from("/api/v1/user/me/preferences/"))
    );

    // `sqlx` logs each statement as a `sqlx::query` event; the values stay bound parameters.
    let has_sql = request.events.iter().any(|event| {
        let has = |key: &str| event.attributes.iter().any(|kv| kv.key.as_str() == key);
        has("db.statement")
            && event
                .attributes
                .iter()
                .any(|kv| kv.key.as_str() == "target" && kv.value == Value::from("sqlx::query"))
    });
    assert!(
        has_sql,
        "the SQL run by the handler must be attached to its span: {:#?}",
        request.events
    );
}

#[tokio::test]
#[serial]
async fn a_refused_request_still_gets_its_span() {
    std::sync::LazyLock::force(&INIT);
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let _subscriber = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(trace_layer(&provider)),
    );

    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);
    exporter.reset();

    // No JWT: `JwtMiddleware` refuses before the handler runs.
    let _ =
        test::try_call_service(&app, test::TestRequest::get().uri(PREFERENCES).to_request()).await;

    provider.force_flush().expect("flush the spans");
    let spans = exporter.get_finished_spans().unwrap();
    let request = spans
        .iter()
        .find(|span| span.name == PREFERENCES_ROUTE)
        .unwrap_or_else(|| panic!("no span named {PREFERENCES_ROUTE}: {spans:#?}"));
    assert_eq!(
        attribute(request, "http.status_code"),
        Some(&Value::I64(401))
    );
}
