use std::path::PathBuf;
use thiserror::Error;

/// Erreurs principales de git_sv.
#[derive(Debug, Error)]
pub enum GitSvError {
    /// Erreur provenant de libgit2
    #[error("Erreur git : {0}")]
    Git(#[from] git2::Error),

    /// Erreur d'entrée/sortie fichier avec contexte
    #[error("Erreur I/O ({context}): {source}")]
    Io {
        source: std::io::Error,
        context: String,
    },

    /// Erreur lors de l'accès au presse-papier
    #[error("Erreur clipboard : {0}")]
    Clipboard(String),

    /// Repository non trouvé ou invalide
    #[error("Repository git non trouvé dans {path}")]
    RepoNotFound { path: PathBuf },

    /// Opération git échouée avec contexte
    #[error("Opération '{operation}' échouée: {details}")]
    OperationFailed {
        operation: &'static str,
        details: String,
    },

    /// Branche non trouvée
    #[error("Branche '{name}' non trouvée")]
    BranchNotFound { name: String },

    /// Fichier non trouvé
    #[error("Fichier '{path}' non trouvé")]
    FileNotFound { path: String },

    /// État invalide de l'application
    #[error("État invalide: {0}")]
    InvalidState(String),

    /// Index hors limites
    #[error("Index {index} hors limites (max: {max})")]
    IndexOutOfBounds { index: usize, max: usize },

    /// Erreur générique
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for GitSvError {
    fn from(err: std::io::Error) -> Self {
        GitSvError::Io {
            source: err,
            context: "I/O operation".to_string(),
        }
    }
}

/// Alias pratique pour Result avec GitSvError.
pub type Result<T> = std::result::Result<T, GitSvError>;

/// Trait d'extension pour ajouter du contexte aux erreurs I/O
pub trait IoErrorContext<T> {
    fn with_context(self, context: impl Into<String>) -> Result<T>;
}

impl<T> IoErrorContext<T> for std::result::Result<T, std::io::Error> {
    fn with_context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|source| GitSvError::Io {
            source,
            context: context.into(),
        })
    }
}
