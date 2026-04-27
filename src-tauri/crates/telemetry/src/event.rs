pub use crate::span::{SpanError, SpanEvent};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_event_creation() {
        let event = SpanEvent::new("test_event");
        assert_eq!(event.name, "test_event");
        assert!(event.attributes.is_empty());
    }

    #[test]
    fn test_span_event_with_attribute() {
        let event = SpanEvent::new("test_event").with_attribute("key", serde_json::json!("value"));
        assert_eq!(
            event.attributes.get("key").unwrap(),
            &serde_json::json!("value")
        );
    }

    #[test]
    fn test_span_error_creation() {
        let error = SpanError::new("TypeError", "Something went wrong");
        assert_eq!(error.error_type, "TypeError");
        assert_eq!(error.message, "Something went wrong");
        assert!(error.stack_trace.is_none());
    }

    #[test]
    fn test_span_error_with_stack_trace() {
        let error =
            SpanError::new("TypeError", "Something went wrong").with_stack_trace("at line 42");
        assert!(error.stack_trace.is_some());
        assert_eq!(error.stack_trace.unwrap(), "at line 42");
    }
}
