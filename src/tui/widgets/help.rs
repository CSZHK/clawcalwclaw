//! Help overlay panel showing keybindings.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

const HELP_TEXT: &str = "\
 Keybindings

 i          Enter editing mode
 Esc        Enter normal mode
 Enter      Send message
 Shift+Enter  Newline in input
 Up/Down    Input history
 PageUp/Dn  Scroll chat
 Ctrl+C     Cancel request (2x to quit)
 Ctrl+D     Quit
 q          Quit (normal mode)
 ?          Toggle this help (normal mode)

 Commands

 /setup-key <KEY>  Set API key inline
 /setup            Show setup help";

pub fn render(frame: &mut Frame<'_>, area: Rect) {
    // Center the overlay in the available area.
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 18.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear background behind the popup.
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help (? to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(HELP_TEXT)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
