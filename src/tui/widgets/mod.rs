//! TUI widget composition and layout.

pub mod approval;
pub mod chat;
pub mod help;
pub mod input;
pub mod sanitize;
pub mod status;
pub mod tools;

use crate::tui::state::TuiState;
use ratatui::prelude::*;

pub fn render(frame: &mut Frame<'_>, state: &TuiState) -> Option<Position> {
    let show_tools = has_visible_tools(state);
    let input_height = input::preferred_height(&state.input_buffer);

    let mut constraints = vec![Constraint::Min(6)];
    if show_tools {
        constraints.push(Constraint::Length(7));
    }
    constraints.push(Constraint::Length(input_height));
    constraints.push(Constraint::Length(1));

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    let mut idx = 0usize;
    chat::render(frame, areas[idx], state);
    idx += 1;
    if show_tools {
        tools::render(frame, areas[idx], state);
        idx += 1;
    }
    let cursor = input::render(frame, areas[idx], state);
    idx += 1;
    status::render(frame, areas[idx], state);

    // ── Overlay layer (rendered on top of everything) ──

    if state.show_help {
        help::render(frame, frame.area());
    }

    if state.pending_approval.is_some() {
        approval::render(frame, frame.area(), state);
    }

    cursor
}

/// Show the tools panel when there are structured tool entries or a non-empty progress block.
fn has_visible_tools(state: &TuiState) -> bool {
    !state.tool_calls.is_empty()
        || state
            .progress_block
            .as_ref()
            .is_some_and(|content| !content.trim().is_empty())
}
