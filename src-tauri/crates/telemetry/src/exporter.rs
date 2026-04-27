use crate::span::{Span, TraceMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceExport {
    pub trace_id: String,
    pub spans: Vec<Span>,
    pub metadata: TraceMetadata,
    pub exported_at: DateTime<Utc>,
}

impl TraceExport {
    pub fn new(trace_id: String, spans: Vec<Span>, metadata: TraceMetadata) -> Self {
        Self {
            trace_id,
            spans,
            metadata,
            exported_at: Utc::now(),
        }
    }
}

pub trait TraceExporter: Send + Sync {
    fn export(&self, trace: TraceExport) -> Result<(), TracerError>;
    fn export_batch(&self, traces: Vec<TraceExport>) -> Result<(), TracerError>;
    fn flush(&self) -> Result<(), TracerError>;
}

#[derive(Debug)]
pub struct TracerError {
    pub message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
}

impl Clone for TracerError {
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            source: None,
        }
    }
}

impl TracerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source<E: Error + Send + Sync + 'static>(mut self, source: E) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl std::fmt::Display for TracerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for TracerError {}

pub struct NoopExporter;

impl TraceExporter for NoopExporter {
    fn export(&self, _trace: TraceExport) -> Result<(), TracerError> {
        Ok(())
    }

    fn export_batch(&self, _traces: Vec<TraceExport>) -> Result<(), TracerError> {
        Ok(())
    }

    fn flush(&self) -> Result<(), TracerError> {
        Ok(())
    }
}

pub struct ConsoleExporter {
    include_inputs: bool,
    include_outputs: bool,
}

impl ConsoleExporter {
    pub fn new() -> Self {
        Self {
            include_inputs: false,
            include_outputs: false,
        }
    }

    pub fn with_io(mut self, include_inputs: bool, include_outputs: bool) -> Self {
        self.include_inputs = include_inputs;
        self.include_outputs = include_outputs;
        self
    }
}

impl Default for ConsoleExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceExporter for ConsoleExporter {
    fn export(&self, trace: TraceExport) -> Result<(), TracerError> {
        println!("=== Trace {} ===", trace.trace_id);
        println!("Metadata: {:?}", trace.metadata);
        for span in &trace.spans {
            println!(
                "  Span [{}] {} ({:?}) - {:?}ms",
                span.id, span.name, span.span_type, span.duration_ms
            );
            if let Some(inputs) = &span.inputs {
                if self.include_inputs {
                    println!("    Inputs: {}", inputs);
                }
            }
            if let Some(outputs) = &span.outputs {
                if self.include_outputs {
                    println!("    Outputs: {}", outputs);
                }
            }
            for error in &span.errors {
                println!("    Error: {} - {}", error.error_type, error.message);
            }
        }
        Ok(())
    }

    fn export_batch(&self, traces: Vec<TraceExport>) -> Result<(), TracerError> {
        for trace in traces {
            self.export(trace)?;
        }
        Ok(())
    }

    fn flush(&self) -> Result<(), TracerError> {
        Ok(())
    }
}
