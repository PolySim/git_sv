# STEP 02 - Amélioration du Système d'Erreurs et Nettoyage du Code Mort

**Priorité**: 🔴 Haute  
**Effort estimé**: 2-3 heures  
**Risque**: Faible à moyen  
**Prérequis**: STEP_01 complété

---

## Objectif

1. Améliorer la gestion des erreurs avec des messages contextuels
2. Supprimer le code mort identifié
3. Remplacer les `unwrap_or_default()` problématiques par une gestion d'erreur explicite
4. Standardiser les patterns de propagation d'erreur

---

## 1. Amélioration de `GitSvError`

### Fichier: `src/error.rs`

```rust
// AVANT
#[derive(Debug, Error)]
pub enum GitSvError {
    #[error("Erreur git : {0}")]
    Git(#[from] git2::Error),

    #[error("Erreur I/O : {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur terminal : {0}")]
    Terminal(String),  // ❌ Jamais utilisé

    #[error("Erreur clipboard : {0}")]
    Clipboard(String),
}

// APRÈS - Version enrichie avec contexte
use std::path::PathBuf;

#[derive(Debug, Error)]
pub enum GitSvError {
    /// Erreur provenant de libgit2
    #[error("Erreur git : {0}")]
    Git(#[from] git2::Error),

    /// Erreur d'entrée/sortie fichier
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
}

/// Trait d'extension pour ajouter du contexte aux erreurs I/O
pub trait IoErrorContext<T> {
    fn with_context(self, context: impl Into<String>) -> Result<T, GitSvError>;
}

impl<T> IoErrorContext<T> for std::result::Result<T, std::io::Error> {
    fn with_context(self, context: impl Into<String>) -> Result<T, GitSvError> {
        self.map_err(|source| GitSvError::Io {
            source,
            context: context.into(),
        })
    }
}
```

---

## 2. Supprimer les `unwrap_or_default()` problématiques

Plus de 200 occurrences de `unwrap_or_default()` ont été identifiées. Certaines sont légitimes, d'autres masquent des erreurs.

### Pattern 1: Graph building - `src/event.rs:1538`

```rust
// AVANT - L'erreur est silencieusement ignorée
self.state.graph = self.state.repo.build_graph(MAX_COMMITS).unwrap_or_default();

// APRÈS - Logger ou signaler l'erreur
match self.state.repo.build_graph(MAX_COMMITS) {
    Ok(graph) => self.state.graph = graph,
    Err(e) => {
        self.state.set_flash_message(format!("Erreur chargement graph: {}", e));
        // Garder l'ancien graph plutôt que le vider
    }
}
```

### Pattern 2: Refresh operations - dispersées dans `src/event.rs`

```rust
// Créer une méthode helper pour la gestion cohérente
impl EventHandler {
    /// Rafraîchit le graph avec gestion d'erreur cohérente
    fn refresh_graph_safe(&mut self) {
        match self.state.repo.build_graph(MAX_COMMITS) {
            Ok(graph) => {
                self.state.graph = graph;
                self.state.mark_dirty();
            }
            Err(e) => {
                self.state.set_flash_message(format!("⚠ Refresh échoué: {}", e));
            }
        }
    }
}
```

### Pattern 3: Status refresh

```rust
// AVANT
self.state.staging_state.unstaged = self.state.repo.status_unstaged().unwrap_or_default();
self.state.staging_state.staged = self.state.repo.status_staged().unwrap_or_default();

// APRÈS - Propager l'erreur ou logger
fn refresh_staging_status(&mut self) -> Result<()> {
    self.state.staging_state.unstaged = self.state.repo.status_unstaged()
        .map_err(|e| GitSvError::OperationFailed {
            operation: "status_unstaged",
            details: e.to_string(),
        })?;
    self.state.staging_state.staged = self.state.repo.status_staged()
        .map_err(|e| GitSvError::OperationFailed {
            operation: "status_staged",
            details: e.to_string(),
        })?;
    Ok(())
}
```

---

## 3. Standardiser les flash messages d'erreur

### Créer un module helper: `src/error_display.rs`

```rust
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
    }
}

/// Formate un message de succès
pub fn format_success_message(operation: &str) -> String {
    format!("{} ✓", operation)
}
```

### Utilisation dans les handlers

```rust
// AVANT
if let Err(e) = some_operation() {
    self.state.set_flash_message(format!("Erreur: {}", e));
} else {
    self.state.set_flash_message("Success".into());
}

// APRÈS
use crate::error_display::{format_error_message, format_success_message};

match some_operation() {
    Ok(_) => self.state.set_flash_message(format_success_message("Commit")),
    Err(e) => self.state.set_flash_message(format_error_message(&e)),
}
```

---

## 4. Créer un helper pour les opérations git

### Fichier: `src/git/helpers.rs` (nouveau)

```rust
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
```

---

## 5. Supprimer les champs non utilisés (après analyse)

### Décision pour chaque champ mort

| Champ | Fichier | Décision | Raison |
|-------|---------|----------|--------|
| `BlameLine::author_email` | `blame.rs` | **UTILISER** | Afficher dans tooltip |
| `BlameLine::timestamp` | `blame.rs` | **UTILISER** | Afficher date relative |
| `FileBlame::path` | `blame.rs` | **UTILISER** | Afficher dans titre |
| `BranchInfo::is_remote` | `branch.rs` | **UTILISER** | Filtrer branches locales/remotes |
| `BranchInfo::last_commit_date` | `branch.rs` | **UTILISER** | Trier par date |

### Fichier: `src/ui/blame_view.rs` - Utiliser les champs

```rust
// Ajouter l'affichage du timestamp dans blame_view
fn render_blame_line(blame_line: &BlameLine, ...) -> Line<'static> {
    let relative_time = crate::utils::time::format_relative_time(blame_line.timestamp);
    
    Line::from(vec![
        Span::styled(format!("{:>4} ", blame_line.line_no), Style::default().dim()),
        Span::styled(
            format!("{:<12}", truncate_str(&blame_line.author, 12)),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!(" {} ", relative_time),  // ← Nouveau: date relative
            Style::default().dim(),
        ),
        Span::raw(&blame_line.content),
    ])
}
```

---

## 6. Fichiers à créer/modifier

| Action | Fichier | Description |
|--------|---------|-------------|
| Modifier | `src/error.rs` | Enrichir `GitSvError` |
| Créer | `src/error_display.rs` | Formatage des erreurs |
| Créer | `src/git/helpers.rs` | Macros et helpers git |
| Modifier | `src/event.rs` | Remplacer `unwrap_or_default` |
| Modifier | `src/ui/blame_view.rs` | Utiliser champs `BlameLine` |
| Modifier | `src/ui/branches_view.rs` | Utiliser champs `BranchInfo` |

---

## 7. Pattern de gestion d'erreur recommandé

```rust
// Dans les handlers event.rs

/// Pattern standard pour les opérations git
fn handle_some_git_operation(&mut self) -> Result<()> {
    // 1. Valider les préconditions
    let branch = self.state.current_branch
        .as_ref()
        .ok_or_else(|| GitSvError::InvalidState("Aucune branche courante".into()))?;
    
    // 2. Exécuter l'opération
    let result = self.state.repo.some_operation(branch)
        .map_err(|e| GitSvError::OperationFailed {
            operation: "some_operation",
            details: e.to_string(),
        })?;
    
    // 3. Mettre à jour l'état
    self.state.update_from(result);
    self.state.set_flash_message(format_success_message("Opération"));
    self.refresh()?;
    
    Ok(())
}
```

---

## 8. Checklist de validation

```bash
# 1. Vérifier la compilation
cargo build

# 2. Vérifier les tests
cargo test

# 3. S'assurer qu'aucun unwrap_or_default problématique ne reste
grep -r "unwrap_or_default" src/ | grep -v "test"

# 4. Vérifier clippy
cargo clippy --all-features -- -D warnings

# 5. Test manuel des cas d'erreur
# - Ouvrir un dossier qui n'est pas un repo git
# - Essayer de push sans remote
# - Essayer de checkout une branche inexistante
```

---

## Notes

- **Ne pas supprimer** les champs morts sans d'abord vérifier s'ils devraient être utilisés
- Les erreurs doivent toujours être **affichées à l'utilisateur** via flash message
- Préférer **propager les erreurs** (`?`) plutôt que les ignorer silencieusement
- Garder une cohérence dans le format des messages (emoji + texte)
