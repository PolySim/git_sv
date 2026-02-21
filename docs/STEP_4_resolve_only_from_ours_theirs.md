# STEP 4 - Refonte UX : résolution uniquement depuis les panneaux Ours/Theirs

## Problème

Actuellement, les raccourcis `o`/`t`/`b` fonctionnent depuis n'importe quel panneau. Le comportement attendu est :

1. **On ne peut choisir `o`/`t`/`b` que quand on est focalisé sur le panneau Ours ou Theirs.**
2. **Les flèches dans ces panneaux permettent de se déplacer entre sections (mode bloc) ou lignes (mode ligne)** pour voir la sélection courante se déplacer visuellement.
3. **Le panneau FileList et le panneau Résultat ne doivent pas permettre `o`/`t`/`b`.**

## Contexte actuel

- Les actions `ConflictChooseOurs`, `ConflictChooseTheirs`, `ConflictChooseBoth` sont mappées inconditionnellement dans `map_conflicts_key()` (`src/ui/input.rs` ligne ~417-419).
- Les handlers correspondants dans `event.rs` (lignes ~2245-2280) appliquent la résolution sur `file.conflicts[section_selected]` sans vérifier le `panel_focus`.
- La navigation par flèches (sections/lignes) ne met pas en évidence visuellement la section/ligne sélectionnée de manière suffisamment claire.

## Fichiers à modifier

| Fichier | Rôle |
|---------|------|
| `src/ui/input.rs` | Conditionner `o`/`t`/`b` au `panel_focus` |
| `src/event.rs` | Aucune modification structurelle nécessaire (les handlers restent) |
| `src/ui/conflicts_view.rs` | Améliorer la mise en évidence de la section/ligne sélectionnée |

## Modifications détaillées

### 1. `src/ui/input.rs` — `map_conflicts_key()`

Conditionner les résolutions au panneau actif. Les touches `o`, `t`, `b` ne produisent une action **que si** `panel_focus` est `OursPanel` ou `TheirsPanel` :

```rust
let panel_focus = state.conflicts_state.as_ref().map(|s| s.panel_focus);

KeyCode::Char('o') => {
    match panel_focus {
        Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
            Some(AppAction::ConflictChooseOurs)
        }
        _ => None,
    }
}
KeyCode::Char('t') => {
    match panel_focus {
        Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
            Some(AppAction::ConflictChooseTheirs)
        }
        _ => None,
    }
}
KeyCode::Char('b') => {
    match panel_focus {
        Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
            Some(AppAction::ConflictChooseBoth)
        }
        _ => None,
    }
}
```

### 2. `src/ui/conflicts_view.rs` — Mise en évidence visuelle

#### a) Panneau Ours — `build_ours_content()`

En mode **bloc**, la section courante (`state.section_selected`) doit être clairement identifiée. Ajouter un indicateur visuel fort :

- La section active a un background `DarkGray` sur toutes ses lignes (pas seulement un marqueur textuel).
- Les sections non sélectionnées gardent le style normal.

```rust
// En mode bloc, si c'est la section sélectionnée et qu'on est sur le panneau Ours
let is_active = state.panel_focus == ConflictPanelFocus::OursPanel;
let bg = if is_active { Color::DarkGray } else { Color::Reset };

for line in &section.ours {
    lines.push(Line::from(Span::styled(
        format!("  {}", line),
        Style::default().fg(Color::Green).bg(bg),
    )));
}
```

En mode **ligne**, la ligne sélectionnée (`state.line_selected`) a un curseur `>` et un background. Les autres lignes du bloc ont un style normal :

```rust
for (idx, line) in section.ours.iter().enumerate() {
    let is_current = state.line_selected == idx;
    let (prefix, style) = if is_current && is_active {
        ("> ", Style::default().fg(Color::Green).bg(Color::DarkGray).add_modifier(Modifier::BOLD))
    } else {
        ("  ", Style::default().fg(Color::Green))
    };
    lines.push(Line::from(Span::styled(format!("{}{}", prefix, line), style)));
}
```

#### b) Panneau Theirs — `build_theirs_content()`

Idem que ours, avec `Color::Blue` et `ConflictPanelFocus::TheirsPanel`.

#### c) Multi-sections

Actuellement, les panneaux ours/theirs ne montrent que **la section sélectionnée** (`file.conflicts.get(state.section_selected)`). Il serait plus clair d'afficher **toutes les sections** avec la section sélectionnée mise en évidence (scroll automatique vers elle). Cela demande de refactorer `render_ours_theirs_panels()` pour itérer sur toutes les sections :

```rust
for (idx, section) in file.conflicts.iter().enumerate() {
    let is_selected = idx == state.section_selected;
    // Afficher le contexte avant
    // Afficher les lignes ours avec bg si is_selected
    // Afficher le contexte après
}
```

Et utiliser `Paragraph::scroll()` avec `state.ours_scroll` pour centrer sur la section active.

### 3. Help bar et overlay

Mettre à jour le texte d'aide pour indiquer que `o`/`t`/`b` ne fonctionnent que dans les panneaux ours/theirs :

```
"o:ours  t:theirs  b:both  (uniquement dans les panneaux ours/theirs)"
```

## Résultat attendu

- `o`/`t`/`b` ne font rien dans FileList et Result.
- La section/ligne sélectionnée est visuellement évidente avec un background.
- En mode ligne, un curseur `>` montre la ligne exacte.
- Les flèches déplacent la sélection visuellement dans le panneau actif.
