use anyhow::{Context, Result};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{RandomIdGenerator, Sampler},
    Resource,
};

/// OpenTelemetry tracing manager for distributed tracing
pub struct TracingManager;

impl TracingManager {
    /// Initialize OpenTelemetry with OTLP exporter
    pub fn new(
        otlp_endpoint: &str,
        service_name: &str,
        sample_rate: f64,
    ) -> Result<Self> {
        // Create OTLP exporter
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(otlp_endpoint);

        // Create tracer provider
        let _tracer_provider = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default()
                    .with_sampler(if sample_rate >= 1.0 {
                        Sampler::AlwaysOn
                    } else if sample_rate <= 0.0 {
                        Sampler::AlwaysOff
                    } else {
                        Sampler::TraceIdRatioBased(sample_rate)
                    })
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", service_name.to_string()),
                        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    ])),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .context("Failed to install OpenTelemetry tracer provider")?;

        // Note: In this version of opentelemetry, install_batch() returns a Tracer
        // The provider is automatically set globally during installation

        tracing::info!(
            "OpenTelemetry initialized: endpoint={}, service={}, sample_rate={}",
            otlp_endpoint,
            service_name,
            sample_rate
        );

        Ok(Self)
    }

    /// Shutdown OpenTelemetry and flush remaining spans
    pub fn shutdown() -> Result<()> {
        global::shutdown_tracer_provider();
        Ok(())
    }
}

/// Helper function to extract trace context from HTTP headers
pub fn extract_trace_context(headers: &hyper::HeaderMap) -> Option<opentelemetry::Context> {
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    let propagator = TraceContextPropagator::new();
    let context = propagator.extract(&HeaderExtractor(headers));

    Some(context)
}

/// Helper function to inject trace context into HTTP headers
pub fn inject_trace_context(
    headers: &mut hyper::HeaderMap,
    context: &opentelemetry::Context,
) {
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    let propagator = TraceContextPropagator::new();
    propagator.inject_context(context, &mut HeaderInjector(headers));
}

/// Helper struct for extracting trace context from HTTP headers
struct HeaderExtractor<'a>(&'a hyper::HeaderMap);

impl<'a> opentelemetry::propagation::Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// Helper struct for injecting trace context into HTTP headers
struct HeaderInjector<'a>(&'a mut hyper::HeaderMap);

impl<'a> opentelemetry::propagation::Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(header_name) = hyper::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(header_value) = hyper::header::HeaderValue::from_str(&value) {
                self.0.insert(header_name, header_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_manager_initialization() {
        // This test would need an actual OTLP collector to work
        // In a real scenario, you would use a test collector or mock
    }
}
