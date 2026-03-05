//! TUI event handling integration tests.
//!
//! Tests the event handling logic without real terminal I/O.
//! Focuses on key event routing and state mutations.

#[cfg(feature = "tui-ratatui")]
mod tui_event_handling {
    use clawclawclaw::tui::events::{translate_delta, TuiEvent};
    use clawclawclaw::tui::state::{InputMode, TuiRole, TuiState};

    /// Test that streaming deltas accumulate correctly in state
    #[test]
    fn streaming_deltas_accumulate_in_assistant_message() {
        let mut state = TuiState::new("provider", "model");

        state.start_streaming_assistant();
        state.append_stream_delta("Hello");
        state.append_stream_delta(" ");
        state.append_stream_delta("world!");

        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, TuiRole::Assistant);
        assert_eq!(state.messages[0].content, "Hello world!");
    }

    /// Test that clear sentinel resets streaming message
    #[test]
    fn clear_sentinel_resets_streaming() {
        let mut state = TuiState::new("provider", "model");

        state.start_streaming_assistant();
        state.append_stream_delta("Old content");

        // Simulate clear event using actual sentinel from agent::loop_
        let event = translate_delta("\x00CLEAR\x00".to_string());
        assert_eq!(event, TuiEvent::Clear);

        // Apply clear: should reset the streaming message
        state.start_streaming_assistant();
        assert_eq!(state.messages.len(), 1);
        assert!(state.messages[0].content.is_empty());
    }

    /// Test progress line sets thinking status
    #[test]
    fn progress_line_sets_thinking_status() {
        let mut state = TuiState::new("provider", "model");

        let event = translate_delta("\x00PROGRESS\x00Thinking...\n".to_string());
        match event {
            TuiEvent::ProgressLine { text } => {
                state.set_thinking(Some(text));
            }
            _ => panic!("Expected ProgressLine event"),
        }

        assert!(state.progress_line.is_some());
    }

    /// Test history navigation in editing mode
    #[test]
    fn history_navigation_in_editing_mode() {
        let mut state = TuiState::new("provider", "model");
        state.mode = InputMode::Editing;

        state.note_submitted_input("first command");
        state.note_submitted_input("second command");
        state.note_submitted_input("third command");

        // Navigate back through history
        let prev = state.history_prev();
        assert_eq!(prev.as_deref(), Some("third command"));

        let prev = state.history_prev();
        assert_eq!(prev.as_deref(), Some("second command"));

        // Navigate forward
        let next = state.history_next();
        assert_eq!(next.as_deref(), Some("third command"));
    }

    /// Test duplicate consecutive inputs are not added to history
    #[test]
    fn duplicate_consecutive_inputs_not_duplicated_in_history() {
        let mut state = TuiState::new("provider", "model");

        state.note_submitted_input("same");
        state.note_submitted_input("same");
        state.note_submitted_input("same");

        assert_eq!(state.input_history.len(), 1);
        assert_eq!(state.input_history[0], "same");
    }

    /// Test mode toggle between Normal and Editing
    #[test]
    fn mode_toggle_switches_correctly() {
        let mut state = TuiState::new("provider", "model");
        assert_eq!(state.mode, InputMode::Editing);

        state.toggle_mode();
        assert_eq!(state.mode, InputMode::Normal);

        state.toggle_mode();
        assert_eq!(state.mode, InputMode::Editing);
    }

    /// Test scroll page up/down respects bounds
    #[test]
    fn scroll_respects_saturating_bounds() {
        let mut state = TuiState::new("provider", "model");

        // Can't scroll below 0
        state.scroll_page_down(100);
        assert_eq!(state.scroll_offset, 0);

        // Scroll up works
        state.scroll_page_up(10);
        assert_eq!(state.scroll_offset, 10);

        // Scroll down partially
        state.scroll_page_down(3);
        assert_eq!(state.scroll_offset, 7);

        // Can't scroll below 0
        state.scroll_page_down(100);
        assert_eq!(state.scroll_offset, 0);
    }

    /// Test status transitions
    #[test]
    fn status_transitions_correctly() {
        use clawclawclaw::tui::state::TuiStatus;

        let mut state = TuiState::new("provider", "model");
        assert_eq!(state.status, TuiStatus::Idle);

        state.set_thinking(Some("Processing...".to_string()));
        assert_eq!(state.status, TuiStatus::Thinking);
        assert!(state.progress_line.is_some());

        state.set_tool_running("Running shell command".to_string());
        assert_eq!(state.status, TuiStatus::ToolRunning);
        assert!(state.progress_block.is_some());

        state.set_idle();
        assert_eq!(state.status, TuiStatus::Idle);
    }
}
