use crate::error::{GitSvError, Result};

/// Macro pour wrapper les opérations git avec contexte
#[macro_export]
macro_rules! git_op {
    ($op:expr, $operation_name:literal) => {
        $op.map_err(|e| GitSvError::OperationFailed {
            operation: $operation_name,
            details: e.to_string(),
        })
    };
}

/// Wrapper générique pour les opérations git qui retournent Result
pub fn with_error_context<T, E: std::fmt::Display>(
    result: std::result::Result<T, E>,
    operation: &'static str,
) -> Result<T> {
    result.map_err(|e| GitSvError::OperationFailed {
        operation,
        details: e.to_string(),
    })
}

/// Helper pour les opérations git optionnelles (retournent Option)
pub fn with_optional_context<T, E: std::fmt::Display>(
    result: std::result::Result<Option<T>, E>,
    operation: &'static str,
) -> Result<Option<T>> {
    result.map_err(|e| GitSvError::OperationFailed {
        operation,
        details: e.to_string(),
    })
}
