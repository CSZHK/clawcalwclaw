//! TUI widget composition and layout.

pub mod approval;
pub mod chat;
pub mod help;
pub mod input;
pub mod sanitize;
pub mod status;
pub mod subagents;
pub mod task_board;
pub mod tools;

use crate::tui::state::TuiState;
use ratatui::prelude::*;

pub fn render(frame: &mut Frame<'_>, state: &TuiState) -> Option<Position> {
    let show_tools = has_visible_tools(state);
    let show_task_board = state
        .task_board_view
        .as_ref()
        .is_some_and(|view| view.has_visible_content());
    let show_subagents = state
        .subagent_pane_view
        .as_ref()
        .is_some_and(|view| view.has_visible_content());
    let show_workbench = show_task_board || show_subagents;
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

    let body_area = areas[0];
    let chat_area = if show_workbench {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(body_area);
        render_workbench_sidebar(frame, split[1], state, show_task_board, show_subagents);
        split[0]
    } else {
        body_area
    };

    let mut idx = 1usize;
    chat::render(frame, chat_area, state);
    if show_tools {
        tools::render(frame, areas[idx], state);
        idx += 1;
    }
    let cursor = input::render(frame, areas[idx], state);
    idx += 1;
    status::render(frame, areas[idx], state);

    if state.show_help {
        help::render(frame, frame.area());
    }

    if !state.approval_queue.is_empty() {
        approval::render(frame, frame.area(), state);
    }

    cursor
}

fn render_workbench_sidebar(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    show_task_board: bool,
    show_subagents: bool,
) {
    match (show_task_board, show_subagents) {
        (true, true) => {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);
            task_board::render(frame, split[0], state.task_board_view.as_ref().unwrap());
            subagents::render(frame, split[1], state.subagent_pane_view.as_ref().unwrap());
        }
        (true, false) => task_board::render(frame, area, state.task_board_view.as_ref().unwrap()),
        (false, true) => subagents::render(frame, area, state.subagent_pane_view.as_ref().unwrap()),
        (false, false) => {}
    }
}

/// Show the tools panel when there are structured tool entries or a non-empty progress block.
fn has_visible_tools(state: &TuiState) -> bool {
    !state.tool_calls.is_empty()
        || state
            .progress_block
            .as_ref()
            .is_some_and(|content| !content.trim().is_empty())
}
