//! Tool approval modal overlay.
//!
//! Displays the active approval queue item and allows fail-closed decisions.

use crate::tui::state::{ApprovalQueueStatus, TuiState};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let Some(approval) = state.active_approval() else {
        return;
    };

    let popup_width = 68.min(area.width.saturating_sub(4));
    let popup_height = 13.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let sanitize = super::sanitize::sanitize_text;
    let pending_count = state
        .approval_queue
        .iter()
        .filter(|item| item.status == ApprovalQueueStatus::Pending)
        .count();
    let title = format!(
        " Approval Queue ({pending_count} pending / {} total) ",
        state.approval_queue.len()
    );

    let tool_name = sanitize(&approval.tool_name);
    let args_summary = sanitize(&approval.arguments_summary);
    let requested_at = sanitize(&approval.requested_at);
    let status_copy = approval
        .status_message
        .as_ref()
        .map(|msg| sanitize(msg))
        .unwrap_or_else(|| default_status_copy(approval.status).to_string());

    let action_copy = if approval.status == ApprovalQueueStatus::Pending {
        "Allow this tool call?  [y] Yes  [n] No"
    } else {
        "Press [Esc] or [Enter] to dismiss this approval item."
    };

    let text = format!(
        "Status: {status_copy}\nRequested: {requested_at}\nTool: {tool_name}\n\n{args_summary}\n\n{action_copy}"
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph, popup_area);
}

fn default_status_copy(status: ApprovalQueueStatus) -> &'static str {
    match status {
        ApprovalQueueStatus::Pending => "Pending explicit operator decision",
        ApprovalQueueStatus::Approved => "Approved and forwarded to runtime",
        ApprovalQueueStatus::Denied => "Denied and kept fail-closed",
        ApprovalQueueStatus::Failed => "Approval bridge failed; runtime remains fail-closed",
        ApprovalQueueStatus::Expired => "Approval expired; runtime remains fail-closed",
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
