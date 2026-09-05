//! OpenTelemetry-ready tracing bootstrap for the API process.
//!
//! This module initializes `tracing-subscriber` only. An OTLP exporter can be
//! layered later without changing call sites. Correlate logs and future spans
//! with `request_id`, `order_id`, `payment_id`, and `event_id`.
//!
//! Safety: never record passwords, card secrets, payment tokens, or full
//! street addresses as log or span fields.

/// Installs the process-wide tracing subscriber.
///
/// `RUST_LOG` selects the filter. When unset, the default is `info`.
/// `OTEL_SERVICE_NAME` names this process in the first log line.
pub fn init() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init()
        .ok();

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "market-bot-api".to_owned());
    tracing::info!(
        service = %service_name,
        correlation_fields = "request_id,order_id,payment_id,event_id",
        "telemetry initialized"
    );
}
