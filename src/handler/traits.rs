//! Traits pour les handlers d'actions.

use crate::error::Result;
use crate::state::AppState;

/// Contexte pour l'exécution des handlers.
pub struct HandlerContext<'a> {
    pub state: &'a mut AppState,
}

/// Trait pour les handlers d'actions.
pub trait ActionHandler {
    /// Type d'action géré par ce handler.
    type Action;

    /// Exécute l'action.
    fn handle(&mut self, ctx: &mut HandlerContext, action: Self::Action) -> Result<()>;
}
