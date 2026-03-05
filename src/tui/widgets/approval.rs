//! Tool approval modal overlay.
//!
//! Displays a confirmation prompt when the agent requests to execute a tool
//! that requires user approval (e.g. shell commands).

use crate::tui::state::TuiState;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let Some(approval) = &state.pending_approval else {
        return;
    };

    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 10.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let sanitize = super::sanitize::sanitize_text;
    let tool_name = sanitize(&approval.tool_name);
    let args_summary = sanitize(&approval.arguments_summary);

    let text = format!(
        " Tool: {tool_name}\n\
         \n\
         {args_summary}\n\
         \n\
         Allow this tool call?  [y] Yes  [n] No"
    );

    let block = Block::default()
        .title(" Approval Required ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
