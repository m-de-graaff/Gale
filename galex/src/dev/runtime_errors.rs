//! Runtime error helpers for dev mode (GX1900-GX1911).
//!
//! Factory functions that create [`Diagnostic`] values for runtime errors
//! detected during development. These are displayed in the browser
//! dev overlay and the terminal.

use crate::errors::{codes, Diagnostic, ErrorCode};
use crate::span::Span;

/// Create a runtime diagnostic for the dev overlay.
fn runtime_error(code: &'static ErrorCode, message: impl Into<String>) -> Diagnostic {
    Diagnostic::with_message(code, message, Span::dummy())
}

/// GX1900: Unhandled action error.
pub fn unhandled_action_error(action_name: &str, error_msg: &str) -> Diagnostic {
    runtime_error(
        &codes::GX1900,
        format!("Unhandled action error in `{}`: {}", action_name, error_msg),
    )
}

/// GX1901: Action returned non-serializable value.
pub fn action_non_serializable(action_name: &str) -> Diagnostic {
    runtime_error(
        &codes::GX1901,
        format!("Action `{}` returned non-serializable value", action_name),
    )
}

/// GX1902: Hydration mismatch between server HTML and client.
pub fn hydration_mismatch(details: &str) -> Diagnostic {
    runtime_error(&codes::GX1902, format!("Hydration mismatch: {}", details))
}

/// GX1903: Signal updated during render.
pub fn signal_update_during_render(signal_name: &str) -> Diagnostic {
    runtime_error(
        &codes::GX1903,
        format!("Signal `{}` updated during render", signal_name),
    )
}

/// GX1904: Effect threw an error.
pub fn effect_threw(error_msg: &str) -> Diagnostic {
    runtime_error(
        &codes::GX1904,
        format!("Effect threw an error: {}", error_msg),
    )
}

/// GX1905: Channel connection failed.
pub fn channel_connection_failed(channel_name: &str, reason: &str) -> Diagnostic {
    runtime_error(
        &codes::GX1905,
        format!("Channel `{}` connection failed: {}", channel_name, reason),
    )
}

/// GX1906: Query fetch failed.
pub fn query_fetch_failed(url: &str, status: u16) -> Diagnostic {
    runtime_error(
        &codes::GX1906,
        format!("Query fetch failed: {} {}", status, url),
    )
}

/// GX1907: Guard runtime validation failed.
pub fn guard_runtime_validation_failed(guard_name: &str, error_msg: &str) -> Diagnostic {
    runtime_error(
        &codes::GX1907,
        format!("Guard `{}` validation failed: {}", guard_name, error_msg),
    )
}

/// GX1908: Slow action (warning).
pub fn slow_action(action_name: &str, duration_secs: f64) -> Diagnostic {
    runtime_error(
        &codes::GX1908,
        format!("Slow action: `{}` took {:.1}s", action_name, duration_secs),
    )
}

/// GX1909: Memory usage high (warning).
pub fn memory_high(usage_mb: f64) -> Diagnostic {
    runtime_error(
        &codes::GX1909,
        format!("Memory usage high: {:.0}MB", usage_mb),
    )
}

/// GX1910: Maximum re-render depth exceeded.
pub fn max_rerender_depth() -> Diagnostic {
    runtime_error(
        &codes::GX1910,
        "Maximum re-render depth exceeded (100 synchronous updates)".to_string(),
    )
}

/// GX1911: Duplicate key in each block.
pub fn duplicate_key(key_value: &str) -> Diagnostic {
    runtime_error(
        &codes::GX1911,
        format!("Duplicate `key` in `each` block: `{}`", key_value),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unhandled_action_error_message() {
        let diag = unhandled_action_error("createUser", "database timeout");
        assert_eq!(diag.code.code, 1900);
        assert!(diag.message.contains("createUser"));
        assert!(diag.message.contains("database timeout"));
        assert!(diag.is_error());
    }

    #[test]
    fn hydration_mismatch_message() {
        let diag = hydration_mismatch("server rendered 3 items, client has 4");
        assert_eq!(diag.code.code, 1902);
        assert!(diag.message.contains("server rendered"));
    }

    #[test]
    fn slow_action_is_warning() {
        let diag = slow_action("heavyQuery", 12.3);
        assert_eq!(diag.code.code, 1908);
        assert!(diag.is_warning());
        assert!(diag.message.contains("12.3s"));
    }

    #[test]
    fn memory_high_is_warning() {
        let diag = memory_high(512.0);
        assert_eq!(diag.code.code, 1909);
        assert!(diag.is_warning());
        assert!(diag.message.contains("512MB"));
    }

    #[test]
    fn max_rerender_depth_message() {
        let diag = max_rerender_depth();
        assert_eq!(diag.code.code, 1910);
        assert!(diag.message.contains("100 synchronous"));
    }

    #[test]
    fn duplicate_key_message() {
        let diag = duplicate_key("user-42");
        assert_eq!(diag.code.code, 1911);
        assert!(diag.message.contains("user-42"));
    }

    #[test]
    fn query_fetch_failed_message() {
        let diag = query_fetch_failed("/api/users", 500);
        assert_eq!(diag.code.code, 1906);
        assert!(diag.message.contains("500"));
        assert!(diag.message.contains("/api/users"));
    }

    #[test]
    fn channel_connection_failed_message() {
        let diag = channel_connection_failed("Chat", "WebSocket refused");
        assert_eq!(diag.code.code, 1905);
        assert!(diag.message.contains("Chat"));
        assert!(diag.message.contains("WebSocket refused"));
    }
}
