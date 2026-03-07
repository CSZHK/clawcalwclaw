//! Read-only task board pane.

use crate::tui::projections::{TaskBoardItem, TaskBoardStatus, TaskBoardView};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render(frame: &mut Frame<'_>, area: Rect, view: &TaskBoardView) {
    let sanitize = super::sanitize::sanitize_text;
    let mut lines = Vec::new();

    if let Some(error) = &view.error_summary {
        lines.push(format!("⚠ {}", sanitize(error)));
        lines.push(String::new());
    }

    if view.durable_items.is_empty() && view.session_items.is_empty() {
        lines.push("No goals or session tasks yet.".to_string());
    } else {
        push_group(&mut lines, "Durable Goals", &view.durable_items);
        push_group(&mut lines, "Session Plan", &view.session_items);
    }

    lines.push(String::new());
    lines.push(format!("Refreshed {}", sanitize(&view.refreshed_at)));

    let paragraph = Paragraph::new(lines.join("\n"))
        .block(Block::default().title(" Task Board ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn push_group(lines: &mut Vec<String>, label: &str, items: &[TaskBoardItem]) {
    if items.is_empty() {
        return;
    }

    let sanitize = super::sanitize::sanitize_text;
    lines.push(format!("{}:", sanitize(label)));
    for item in items {
        let priority = item
            .priority_label
            .as_ref()
            .map(|value| format!(" [{}]", sanitize(value)))
            .unwrap_or_default();
        let detail = item
            .detail_summary
            .as_ref()
            .map(|value| format!(" — {}", sanitize(value)))
            .unwrap_or_default();
        lines.push(format!(
            "{} {}{}{}",
            status_badge(item.status),
            sanitize(&item.title),
            priority,
            detail
        ));
    }
    lines.push(String::new());
}

fn status_badge(status: TaskBoardStatus) -> &'static str {
    match status {
        TaskBoardStatus::Pending => "[ ]",
        TaskBoardStatus::InProgress => "[~]",
        TaskBoardStatus::Completed => "[✓]",
        TaskBoardStatus::Failed => "[!]",
        TaskBoardStatus::Blocked => "[x]",
    }
}
