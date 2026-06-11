// SPDX-License-Identifier: AGPL-3.0-only

use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::sync::Arc;

pub struct OtelConfig {
    pub endpoint: String,
    pub service_name: String,
    pub enabled: bool,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4318".to_string(),
            service_name: "axagent".to_string(),
            enabled: false,
        }
    }
}

pub struct OtelProviders {
    pub tracer_provider: Option<Arc<SdkTracerProvider>>,
    pub meter_provider: Option<Arc<SdkMeterProvider>>,
    pub enabled: bool,
}

impl OtelProviders {
    pub fn init(config: &OtelConfig) -> Result<Self, String> {
        if !config.enabled {
            tracing::info!("OpenTelemetry disabled, skipping initialization");
            return Ok(Self {
                tracer_provider: None,
                meter_provider: None,
                enabled: false,
            });
        }

        let resource = Resource::builder()
            .with_attributes([
                KeyValue::new("service.name", config.service_name.clone()),
                KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            ])
            .build();

        let span_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(format!("{}/v1/traces", config.endpoint))
            .build()
            .map_err(|e| format!("Failed to build OTLP span exporter: {}", e))?;

        let tracer_provider = SdkTracerProvider::builder()
            .with_batch_exporter(span_exporter)
            .with_resource(resource.clone())
            .build();

        let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_endpoint(format!("{}/v1/metrics", config.endpoint))
            .build()
            .map_err(|e| format!("Failed to build OTLP metric exporter: {}", e))?;

        let reader = PeriodicReader::builder(metric_exporter).build();

        let meter_provider = SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(resource)
            .build();

        tracing::info!(
            endpoint = %config.endpoint,
            service = %config.service_name,
            "OpenTelemetry initialized"
        );

        Ok(Self {
            tracer_provider: Some(Arc::new(tracer_provider)),
            meter_provider: Some(Arc::new(meter_provider)),
            enabled: true,
        })
    }

    pub fn shutdown(&self) {
        if let Some(ref tracer_provider) = self.tracer_provider
            && let Err(e) = tracer_provider.shutdown()
        {
            tracing::warn!("Failed to shutdown tracer provider: {:?}", e);
        }
        if let Some(ref meter_provider) = self.meter_provider
            && let Err(e) = meter_provider.shutdown()
        {
            tracing::warn!("Failed to shutdown meter provider: {:?}", e);
        }
    }

    pub fn record_error_report(&self, report: &axagent_harness::core_error::ErrorReport) {
        if !self.enabled {
            return;
        }
        tracing::info!(
            error_code = %report.error_code,
            recoverable = report.recoverable,
            component = %report.context.component,
            operation = %report.context.operation,
            "Error report recorded to OpenTelemetry"
        );
    }
}
