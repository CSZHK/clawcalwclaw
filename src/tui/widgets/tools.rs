//! Structured tool execution panel.
//!
//! Renders a table of tool calls with status indicators, replacing the
//! previous raw-text progress block display. Falls back to legacy
//! `progress_block` text rendering when no structured entries exist.

use crate::tui::state::{ToolCallStatus, TuiState};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let sanitize = super::sanitize::sanitize_text;

    let rows: Vec<Row<'_>> = state
        .tool_calls
        .iter()
        .rev()
        .take(max_visible_rows(area))
        .map(|entry| {
            let (icon, duration) = match &entry.status {
                ToolCallStatus::Running => ("\u{23f3}", String::new()), // hourglass
                ToolCallStatus::Success(s) => ("\u{2705}", format!("({s}s)")),
                ToolCallStatus::Failed(s) => ("\u{274c}", format!("({s}s)")),
            };
            let name = sanitize(&entry.name);
            let hint = sanitize(&entry.hint);
            Row::new(vec![
                Cell::from(icon),
                Cell::from(name),
                Cell::from(hint),
                Cell::from(duration),
            ])
        })
        .collect();

    // Fall back to legacy progress_block display if no structured entries exist.
    if rows.is_empty() {
        render_legacy_progress(frame, area, state);
        return;
    }

    let widths = [
        Constraint::Length(2),
        Constraint::Length(14),
        Constraint::Min(10),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths)
        .block(Block::default().title("Tools").borders(Borders::ALL))
        .column_spacing(1);

    frame.render_widget(table, area);
}

/// Legacy fallback: render raw progress_block text (backward compatibility).
fn render_legacy_progress(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let content = state
        .progress_block
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("No active tool execution.");
    let clean = super::chat::sanitize_display(content);
    let panel = Paragraph::new(clean)
        .block(Block::default().title("Tools").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn max_visible_rows(area: Rect) -> usize {
    // Account for block borders (top + bottom = 2 rows).
    area.height.saturating_sub(2) as usize
}
