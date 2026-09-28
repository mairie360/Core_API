//! OpenTelemetry tracing (MAIR-131).
//!
//! Traces are exported over OTLP/HTTP (protobuf) to a collector or agent (`OTel` Collector, Grafana
//! Alloy) that relays them to the observability backend (Scaleway Cockpit). Everything is opt-in
//! and driven by the standard OpenTelemetry environment variables, so a deployment without a
//! collector behaves exactly as before:
//!
//! - `OTEL_EXPORTER_OTLP_ENDPOINT` (or `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`): enables the export.
//!   With the generic variable, `/v1/traces` is appended, e.g. `http://alloy:4318`.
//! - `OTEL_SDK_DISABLED=true`: forces the export off even when an endpoint is set.
//! - `OTEL_SERVICE_NAME` (default `core-api`), `OTEL_RESOURCE_ATTRIBUTES`,
//!   `OTEL_EXPORTER_OTLP_HEADERS`, `OTEL_EXPORTER_OTLP_TIMEOUT`, `OTEL_TRACES_SAMPLER`: read by the
//!   SDK itself.
//! - `RUST_LOG`: filter of the stdout logs (default `info`), started with the export.
//!
//! Each HTTP request gets a root span from `tracing_actix_web::TracingLogger` (it continues the
//! `traceparent` sent by a BFF). The SQL statements run by `mairie360_api_lib` through `sqlx` are
//! attached to it as span events (`db.statement`, `elapsed`, rows) because `sqlx` reports them as
//! `tracing` events of the `sqlx::query` target, not as spans of their own.

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing::Subscriber;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

/// `service.name` reported when `OTEL_SERVICE_NAME` is not set.
pub const DEFAULT_SERVICE_NAME: &str = "core-api";

/// Filter of the trace layer: `sqlx::query` is a `DEBUG` target, everything else stays at `INFO`.
const TRACE_FILTER: &str = "info,sqlx::query=debug";

/// Instrumentation scope name of the tracer.
const TRACER_NAME: &str = "core_api";

/// Keeps the tracer provider alive: dropping it flushes the spans still buffered.
pub struct TelemetryGuard {
    provider: SdkTracerProvider,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Err(error) = self.provider.shutdown() {
            eprintln!("OpenTelemetry shutdown failed: {error}");
        }
    }
}

/// Tells whether the environment asks for the trace export, given a variable lookup.
#[must_use]
pub fn is_enabled(lookup: impl Fn(&str) -> Option<String>) -> bool {
    let set = |name: &str| lookup(name).is_some_and(|value| !value.trim().is_empty());
    let disabled =
        lookup("OTEL_SDK_DISABLED").is_some_and(|value| value.trim().eq_ignore_ascii_case("true"));
    !disabled && (set("OTEL_EXPORTER_OTLP_ENDPOINT") || set("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT"))
}

/// Resource describing this service: `service.name` (`OTEL_SERVICE_NAME` wins over
/// [`DEFAULT_SERVICE_NAME`]) plus whatever `OTEL_RESOURCE_ATTRIBUTES` declares.
fn resource() -> Resource {
    let builder = Resource::builder();
    if std::env::var_os("OTEL_SERVICE_NAME").is_some() {
        builder.build()
    } else {
        builder.with_service_name(DEFAULT_SERVICE_NAME).build()
    }
}

/// Trace layer bridging `tracing` spans to OpenTelemetry through `provider`.
#[must_use]
pub fn trace_layer<S>(provider: &SdkTracerProvider) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    tracing_opentelemetry::layer()
        .with_tracer(provider.tracer(TRACER_NAME))
        .with_filter(EnvFilter::new(TRACE_FILTER))
}

/// Starts the export when the environment asks for it and returns the guard to keep until exit.
///
/// Observability must never take the API down: when the exporter cannot be built, the reason is
/// printed and the API runs without traces.
#[must_use]
pub fn init() -> Option<TelemetryGuard> {
    if !is_enabled(|name| std::env::var(name).ok()) {
        return None;
    }
    let exporter = match SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .build()
    {
        Ok(exporter) => exporter,
        Err(error) => {
            eprintln!("OpenTelemetry disabled: cannot build the OTLP exporter: {error}");
            return None;
        }
    };
    let provider = SdkTracerProvider::builder()
        .with_resource(resource())
        .with_batch_exporter(exporter)
        .build();
    global::set_tracer_provider(provider.clone());
    global::set_text_map_propagator(TraceContextPropagator::new());

    let logs = tracing_subscriber::fmt::layer()
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")));
    if let Err(error) = tracing_subscriber::registry()
        .with(logs)
        .with(trace_layer(&provider))
        .try_init()
    {
        eprintln!("OpenTelemetry disabled: a tracing subscriber is already installed: {error}");
        return None;
    }
    Some(TelemetryGuard { provider })
}

#[cfg(test)]
mod tests {
    use super::is_enabled;
    use std::collections::HashMap;

    fn lookup(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        move |name| vars.get(name).cloned()
    }

    #[test]
    fn disabled_without_an_endpoint() {
        assert!(!is_enabled(lookup(&[])));
        assert!(!is_enabled(lookup(&[(
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "  "
        )])));
    }

    #[test]
    fn enabled_by_the_generic_or_the_traces_endpoint() {
        assert!(is_enabled(lookup(&[(
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "http://alloy:4318"
        )])));
        assert!(is_enabled(lookup(&[(
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            "http://alloy:4318/v1/traces"
        )])));
    }

    #[test]
    fn sdk_disabled_wins_over_the_endpoint() {
        assert!(!is_enabled(lookup(&[
            ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://alloy:4318"),
            ("OTEL_SDK_DISABLED", "TRUE"),
        ])));
        assert!(is_enabled(lookup(&[
            ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://alloy:4318"),
            ("OTEL_SDK_DISABLED", "false"),
        ])));
    }
}
