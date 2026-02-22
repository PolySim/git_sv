# STEP-05 : Désélectionner une ligne/block en re-cliquant sur Enter

## Problème

En mode Block ou Ligne, quand une section/ligne est déjà sélectionnée (résolue) et que l'utilisateur appuie à nouveau sur Enter, ça doit **désélectionner** (annuler la résolution).

## Fichiers à modifier

- `src/handler/conflict.rs`

## Corrections

### 1. Toggle dans `handle_enter_resolve` pour le mode Block

Quand Enter est pressé en mode Block et que la section courante a déjà une résolution correspondant au panneau actif, annuler cette résolution :

```rust
// En mode Block, dans handle_enter_resolve :
ConflictResolutionMode::Block => {
    let section = &mut file.sections[section_idx];
    match conflicts.panel_focus {
        ConflictPanelFocus::OursPanel => {
            if section.resolution == Some(ConflictResolution::Ours) {
                section.resolution = None; // Désélectionner
            } else {
                section.resolution = Some(ConflictResolution::Ours);
            }
        }
        ConflictPanelFocus::TheirsPanel => {
            if section.resolution == Some(ConflictResolution::Theirs) {
                section.resolution = None; // Désélectionner
            } else {
                section.resolution = Some(ConflictResolution::Theirs);
            }
        }
        _ => {}
    }
}
```

### 2. Toggle dans `handle_toggle_line` pour le mode Ligne

C'est déjà le comportement naturel du toggle (flip du booléen `included`). S'assurer que le toggle fonctionne bien dans les deux sens.

### 3. Feedback visuel

Quand une section/ligne est désélectionnée, le rendu dans le panneau résultat doit se mettre à jour pour refléter l'absence de résolution (par ex. réafficher les marqueurs de conflit pour cette section).

## Vérification

```bash
cargo build
# Tester : en mode Block, sélectionner une section Ours avec Enter, puis re-Enter pour désélectionner
# Vérifier que le panneau résultat se met à jour
```
