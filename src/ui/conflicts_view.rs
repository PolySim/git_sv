        .border_style(if state.panel_focus == ConflictPanelFocus::FileList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });