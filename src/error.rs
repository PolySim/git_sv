use thiserror::Error;

/// Erreurs principales de git_sv.
#[derive(Debug, Error)]
pub enum GitSvError {
    #[error("Erreur git : {0}")]
    Git(#[from] git2::Error),

    #[error("Erreur I/O : {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur clipboard : {0}")]
    Clipboard(String),

    #[error("{0}")]
    Other(String),
}

/// Alias pratique pour Result avec GitSvError.
pub type Result<T> = std::result::Result<T, GitSvError>;
