use crate::error::GitSvError;

/// Formate une erreur pour l'affichage utilisateur
pub fn format_error_message(err: &GitSvError) -> String {
    match err {
        GitSvError::Git(e) => format!("❌ Git: {}", e),
        GitSvError::Io { context, source } => format!("❌ I/O ({}): {}", context, source),
        GitSvError::Clipboard(msg) => format!("❌ Presse-papier: {}", msg),
        GitSvError::RepoNotFound { path } => format!("❌ Repo non trouvé: {}", path.display()),
        GitSvError::OperationFailed { operation, details } => {
            format!("❌ {} échoué: {}", operation, details)
        }
        GitSvError::BranchNotFound { name } => format!("❌ Branche '{}' non trouvée", name),
        GitSvError::FileNotFound { path } => format!("❌ Fichier '{}' non trouvé", path),
        GitSvError::InvalidState(msg) => format!("❌ État invalide: {}", msg),
        GitSvError::IndexOutOfBounds { index, max } => {
            format!("❌ Index {} hors limites (max: {})", index, max)
        }
        GitSvError::Other(msg) => format!("❌ {}", msg),
    }
}

/// Formate un message de succès
pub fn format_success_message(operation: &str) -> String {
    format!("{} ✓", operation)
}

/// Formate un message d'information
pub fn format_info_message(msg: &str) -> String {
    format!("ℹ {}", msg)
}

/// Formate un message d'avertissement
pub fn format_warning_message(msg: &str) -> String {
    format!("⚠ {}", msg)
}
