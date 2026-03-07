//! Low-noise sub-agent execution pane.

use crate::tui::projections::{SubAgentPaneView, SubAgentProjectionItem, SubAgentViewStatus};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render(frame: &mut Frame<'_>, area: Rect, view: &SubAgentPaneView) {
    let sanitize = super::sanitize::sanitize_text;
    let mut lines = Vec::new();

    if view.items.is_empty() {
        lines.push("No sub-agents yet.".to_string());
    } else {
        for item in &view.items {
            push_item(&mut lines, item);
            lines.push(String::new());
        }
    }
    lines.push(format!("Refreshed {}", sanitize(&view.refreshed_at)));

    let paragraph = Paragraph::new(lines.join("\n"))
        .block(Block::default().title(" Sub-Agents ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn push_item(lines: &mut Vec<String>, item: &SubAgentProjectionItem) {
    let sanitize = super::sanitize::sanitize_text;
    lines.push(format!(
        "{} {}",
        status_badge(item.status),
        sanitize(&item.agent_name)
    ));
    lines.push(format!("Task: {}", sanitize(&item.task_summary)));

    if let Some(summary) = &item.last_event_summary {
        lines.push(format!("Event: {}", sanitize(summary)));
    }
    if let Some(tool) = &item.last_tool_name {
        lines.push(format!("Tool: {}", sanitize(tool)));
    }
    if let Some(error) = &item.error_summary {
        lines.push(format!("Error: {}", sanitize(error)));
    }
}

fn status_badge(status: SubAgentViewStatus) -> &'static str {
    match status {
        SubAgentViewStatus::Running => "[~]",
        SubAgentViewStatus::Completed => "[✓]",
        SubAgentViewStatus::Failed => "[!]",
        SubAgentViewStatus::Killed => "[x]",
    }
}
