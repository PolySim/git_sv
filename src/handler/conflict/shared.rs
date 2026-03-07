use crate::git::conflict::MergeFile;

/// Ajuste le scroll pour garder la ligne sélectionnée visible.
pub(super) fn adjust_scroll(selected: usize, scroll_offset: usize, visible_height: usize) -> usize {
    if visible_height == 0 {
        return 0;
    }
    if selected < scroll_offset {
        // La ligne est au-dessus de la vue, on remonte
        selected
    } else if selected >= scroll_offset + visible_height {
        // La ligne est en dessous de la vue, on descend
        selected.saturating_add(1).saturating_sub(visible_height)
    } else {
        // La ligne est visible, on ne change rien
        scroll_offset
    }
}

/// Calcule la position absolue d'une ligne dans le contenu rendu d'un panneau.
/// Retourne (ligne_absolue, nombre_total_de_lignes)
pub(super) fn calculate_absolute_line_position(
    file: &MergeFile,
    section_selected: usize,
    line_selected: usize,
    is_file_mode: bool,
) -> (usize, usize) {
    let mut current_line: usize = 0;

    for (idx, section) in file.conflicts.iter().enumerate() {
        // Ajouter le séparateur entre sections (sauf la première)
        if idx > 0 {
            current_line += 1;
        }

        // Ajouter le titre de section (sauf en mode Fichier)
        if !is_file_mode {
            current_line += 1;
        }

        // Ajouter les lignes de contexte avant
        current_line += section.context_before.len();

        // Si c'est la section sélectionnée, on ajoute la ligne sélectionnée
        if idx == section_selected {
            current_line += line_selected;
            break;
        }

        // Sinon, on ajoute toutes les lignes de conflit de cette section
        current_line += section.ours.len();

        // Ajouter les lignes de contexte après
        current_line += section.context_after.len();
    }

    (current_line, current_line + 1)
}

/// Avance à la sélection au prochain fichier non résolu.
pub(super) fn advance_to_next_unresolved(conflicts: &mut crate::state::ConflictsState) {
    let current = conflicts.file_selected;
    let total = conflicts.all_files.len();

    // Chercher un fichier non résolu après le courant
    for i in (current + 1)..total {
        if let Some(file) = conflicts.all_files.get(i) {
            if !file.is_resolved {
                conflicts.file_selected = i;
                conflicts.section_selected = 0;
                return;
            }
        }
    }

    // Si aucun trouvé après, chercher depuis le début
    for i in 0..current {
        if let Some(file) = conflicts.all_files.get(i) {
            if !file.is_resolved {
                conflicts.file_selected = i;
                conflicts.section_selected = 0;
                return;
            }
        }
    }

    // Si tous les fichiers sont résolus, rester sur le courant
}
