//! Utilitaires divers pour l'application.

pub mod time;

pub use time::{format_absolute_time, format_relative_time};

/// Macros de profiling conditionnelles.
///
/// Activer avec: cargo run --features profiling
#[cfg(feature = "profiling")]
#[macro_export]
macro_rules! time_block {
    ($name:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        let result = $block;
        eprintln!("[PERF] {} took {:?}", $name, start.elapsed());
        result
    }};
}

#[cfg(feature = "profiling")]
#[macro_export]
macro_rules! time_fn {
    () => {{
        let start = std::time::Instant::now();
        let result = (|| {
            // Le code de la fonction sera ici
        })();
        eprintln!("[PERF] {} took {:?}", module_path!(), start.elapsed());
        result
    }};
}

/// Version no-op quand profiling est désactivé.
#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! time_block {
    ($name:expr, $block:expr) => {
        $block
    };
}

/// Version no-op quand profiling est désactivé.
#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! time_fn {
    () => {
        // No-op
    };
}
