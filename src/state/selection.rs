//! Gestion générique de sélection dans une liste avec scroll.

use std::ops::{Deref, DerefMut};

/// Gère la sélection et le scroll dans une liste d'éléments.
#[derive(Debug, Clone, Default)]
pub struct ListSelection<T> {
    /// Éléments de la liste (accessible pour compatibilité interne).
    pub(crate) items: Vec<T>,
    selected: usize,
    scroll_offset: usize,
    visible_height: usize,
}

impl<T> ListSelection<T> {
    /// Crée une nouvelle sélection vide.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            visible_height: 10, // Valeur par défaut
        }
    }

    /// Crée une sélection avec des éléments.
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            items,
            selected: 0,
            scroll_offset: 0,
            visible_height: 10,
        }
    }

    /// Définit la hauteur visible (pour le scroll).
    pub fn set_visible_height(&mut self, height: usize) {
        self.visible_height = height;
        self.adjust_scroll();
    }

    /// Remplace les éléments.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        // Ajuster la sélection si nécessaire
        if self.selected >= self.items.len() && !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
        self.adjust_scroll();
    }

    /// Index de l'élément sélectionné.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Élément actuellement sélectionné.
    pub fn selected_item(&self) -> Option<&T> {
        self.items.get(self.selected)
    }

    /// Élément actuellement sélectionné (mutable).
    pub fn selected_item_mut(&mut self) -> Option<&mut T> {
        self.items.get_mut(self.selected)
    }

    /// Offset de scroll actuel.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Nombre d'éléments.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// La liste est-elle vide?
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Sélectionne l'élément précédent.
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.adjust_scroll();
        }
    }

    /// Sélectionne l'élément suivant.
    pub fn select_next(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
            self.adjust_scroll();
        }
    }

    /// Remonte d'une page.
    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(self.visible_height);
        self.adjust_scroll();
    }

    /// Descend d'une page.
    pub fn page_down(&mut self) {
        self.selected = (self.selected + self.visible_height).min(
            self.items.len().saturating_sub(1)
        );
        self.adjust_scroll();
    }

    /// Va au premier élément.
    pub fn select_first(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Va au dernier élément.
    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.selected = self.items.len() - 1;
            self.adjust_scroll();
        }
    }

    /// Sélectionne un index spécifique.
    pub fn select(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected = index;
            self.adjust_scroll();
        }
    }

    /// Ajuste le scroll pour garder la sélection visible.
    fn adjust_scroll(&mut self) {
        // La sélection est au-dessus de la zone visible
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
        // La sélection est en-dessous de la zone visible
        if self.selected >= self.scroll_offset + self.visible_height {
            self.scroll_offset = self.selected - self.visible_height + 1;
        }
    }

    /// Itère sur les éléments visibles avec leur index original.
    pub fn visible_items(&self) -> impl Iterator<Item = (usize, &T)> {
        self.items
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.visible_height)
    }
}

impl<T> Deref for ListSelection<T> {
    type Target = Vec<T>;
    
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T> DerefMut for ListSelection<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl<'a, T> IntoIterator for &'a ListSelection<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<T> IntoIterator for ListSelection<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_next() {
        let mut sel = ListSelection::with_items(vec![1, 2, 3, 4, 5]);
        assert_eq!(sel.selected_index(), 0);
        
        sel.select_next();
        assert_eq!(sel.selected_index(), 1);
        
        sel.select_next();
        sel.select_next();
        sel.select_next();
        assert_eq!(sel.selected_index(), 4);
        
        // Ne dépasse pas la fin
        sel.select_next();
        assert_eq!(sel.selected_index(), 4);
    }

    #[test]
    fn test_scroll_adjustment() {
        let mut sel = ListSelection::with_items(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        sel.set_visible_height(3);
        
        assert_eq!(sel.scroll_offset(), 0);
        
        sel.select(5);
        assert!(sel.scroll_offset() > 0);
    }

    #[test]
    fn test_empty_list() {
        let mut sel: ListSelection<i32> = ListSelection::new();
        sel.select_next();
        sel.select_previous();
        assert_eq!(sel.selected_index(), 0);
        assert!(sel.selected_item().is_none());
    }
}
