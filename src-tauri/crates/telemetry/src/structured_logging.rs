use std::cell::RefCell;

thread_local! {
    static CURRENT_TRACE_ID: RefCell<Option<String>> = const { RefCell::new(None) };
    static CURRENT_SESSION_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

static GLOBAL_TRACE_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub fn generate_trace_id() -> String {
    let counter = GLOBAL_TRACE_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let timestamp = chrono::Utc::now().timestamp_millis();
    format!("trace-{}-{}", timestamp, counter)
}

pub fn set_trace_id(trace_id: String) {
    CURRENT_TRACE_ID.with(|tid| {
        *tid.borrow_mut() = Some(trace_id);
    });
}

pub fn get_trace_id() -> Option<String> {
    CURRENT_TRACE_ID.with(|tid| tid.borrow().clone())
}

pub fn set_session_id(session_id: String) {
    CURRENT_SESSION_ID.with(|sid| {
        *sid.borrow_mut() = Some(session_id);
    });
}

pub fn get_session_id() -> Option<String> {
    CURRENT_SESSION_ID.with(|sid| sid.borrow().clone())
}

pub fn clear_context() {
    CURRENT_TRACE_ID.with(|tid| {
        *tid.borrow_mut() = None;
    });
    CURRENT_SESSION_ID.with(|sid| {
        *sid.borrow_mut() = None;
    });
}

#[macro_export]
macro_rules! structured_span {
    ($level:expr, $name:expr) => {
        {
            let trace_id = $crate::structured_logging::get_trace_id().unwrap_or_else(|| "no-trace".to_string());
            let session_id = $crate::structured_logging::get_session_id().unwrap_or_else(|| "no-session".to_string());
            tracing::span!($level, $name, trace_id = %trace_id, session_id = %session_id)
        }
    };
}

#[macro_export]
macro_rules! structured_info {
    ($($arg:tt)*) => {
        {
            let trace_id = $crate::structured_logging::get_trace_id().unwrap_or_else(|| "no-trace".to_string());
            let session_id = $crate::structured_logging::get_session_id().unwrap_or_else(|| "no-session".to_string());
            tracing::info!(trace_id = %trace_id, session_id = %session_id, $($arg)*)
        }
    };
}

#[macro_export]
macro_rules! structured_warn {
    ($($arg:tt)*) => {
        {
            let trace_id = $crate::structured_logging::get_trace_id().unwrap_or_else(|| "no-trace".to_string());
            let session_id = $crate::structured_logging::get_session_id().unwrap_or_else(|| "no-session".to_string());
            tracing::warn!(trace_id = %trace_id, session_id = %session_id, $($arg)*)
        }
    };
}

#[macro_export]
macro_rules! structured_error {
    ($($arg:tt)*) => {
        {
            let trace_id = $crate::structured_logging::get_trace_id().unwrap_or_else(|| "no-trace".to_string());
            let session_id = $crate::structured_logging::get_session_id().unwrap_or_else(|| "no-session".to_string());
            tracing::error!(trace_id = %trace_id, session_id = %session_id, $($arg)*)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_generation() {
        let id1 = generate_trace_id();
        let id2 = generate_trace_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("trace-"));
    }

    #[test]
    fn test_set_get_trace_id() {
        set_trace_id("test-trace-123".to_string());
        assert_eq!(get_trace_id(), Some("test-trace-123".to_string()));
        clear_context();
        assert_eq!(get_trace_id(), None);
    }

    #[test]
    fn test_set_get_session_id() {
        set_session_id("session-456".to_string());
        assert_eq!(get_session_id(), Some("session-456".to_string()));
        clear_context();
        assert_eq!(get_session_id(), None);
    }
}
