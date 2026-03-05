//! Single-line status bar with provider, model, tokens, cost, and status.

use crate::tui::state::{TuiState, TuiStatus};
use ratatui::{prelude::*, widgets::Paragraph};

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let status_text = match state.status {
        TuiStatus::Idle => "idle",
        TuiStatus::Thinking => "thinking",
        TuiStatus::ToolRunning => "tool-running",
    };
    let total_tokens = state
        .session_input_tokens
        .saturating_add(state.session_output_tokens);
    let tokens_display = format_token_count(total_tokens);
    let cost_display = format_cost(state.session_cost_usd);
    let line = format!(
        "{} | {} | {tokens_display} | {cost_display} | {status_text} | {:?}",
        state.provider_id, state.model_id, state.mode
    );
    let widget = Paragraph::new(line).style(Style::default().fg(Color::Gray));
    frame.render_widget(widget, area);
}

fn format_token_count(count: u64) -> String {
    if count == 0 {
        "0 tokens".to_string()
    } else if count >= 1_000_000 {
        format!("{:.1}M tokens", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k tokens", count as f64 / 1_000.0)
    } else {
        format!("{count} tokens")
    }
}

fn format_cost(cost: f64) -> String {
    if cost < 0.001 {
        "$0.00".to_string()
    } else {
        format!("${cost:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::{format_cost, format_token_count};

    #[test]
    fn token_count_formatting() {
        assert_eq!(format_token_count(0), "0 tokens");
        assert_eq!(format_token_count(500), "500 tokens");
        assert_eq!(format_token_count(12_345), "12.3k tokens");
        assert_eq!(format_token_count(1_500_000), "1.5M tokens");
    }

    #[test]
    fn cost_formatting() {
        assert_eq!(format_cost(0.0), "$0.00");
        assert_eq!(format_cost(0.0001), "$0.00");
        assert_eq!(format_cost(0.04), "$0.04");
        assert_eq!(format_cost(1.2345), "$1.23");
    }
}
