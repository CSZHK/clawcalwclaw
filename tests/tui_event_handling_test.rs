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

    /// Test structured tool call lifecycle via state
    #[test]
    fn structured_tool_lifecycle_through_state() {
        use clawclawclaw::tui::state::ToolCallStatus;

        let mut state = TuiState::new("provider", "model");

        // Start two tools
        state.add_tool_start(0, "shell".to_string(), "ls -la".to_string());
        state.add_tool_start(1, "file_read".to_string(), "src/main.rs".to_string());
        assert_eq!(state.tool_calls.len(), 2);
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Running);
        assert_eq!(state.tool_calls[1].status, ToolCallStatus::Running);

        // Complete one successfully, one with failure
        state.complete_tool(0, true, 3);
        state.complete_tool(1, false, 1);
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Success(3));
        assert_eq!(state.tool_calls[1].status, ToolCallStatus::Failed(1));

        // Clear all
        state.clear_tool_calls();
        assert!(state.tool_calls.is_empty());
    }

    /// Test parallel same-name tool calls complete by progress ID.
    #[test]
    fn parallel_same_name_tools_complete_by_progress_id() {
        use clawclawclaw::tui::state::ToolCallStatus;

        let mut state = TuiState::new("provider", "model");

        // Two shell calls running simultaneously with different progress IDs.
        state.add_tool_start(10, "shell".to_string(), "cmd-1".to_string());
        state.add_tool_start(11, "shell".to_string(), "cmd-2".to_string());

        // Completion must match the exact progress ID, not the shared tool name.
        state.complete_tool(10, true, 5);
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Success(5));
        assert_eq!(state.tool_calls[1].status, ToolCallStatus::Running);

        state.complete_tool(11, true, 7);
        assert_eq!(state.tool_calls[1].status, ToolCallStatus::Success(7));
    }

    /// Test usage accumulation across multiple LLM calls
    #[test]
    fn usage_accumulation_across_multiple_calls() {
        let mut state = TuiState::new("provider", "model");

        state.accumulate_usage(Some(100), Some(50), Some(0.01));
        state.accumulate_usage(Some(200), Some(100), Some(0.02));
        state.accumulate_usage(None, None, None); // no-op
        state.accumulate_usage(Some(50), None, Some(0.005));

        assert_eq!(state.session_input_tokens, 350);
        assert_eq!(state.session_output_tokens, 150);
        assert!((state.session_cost_usd - 0.035).abs() < 1e-10);
    }

    /// Test help toggle does not interfere with other state
    #[test]
    fn help_toggle_isolated_from_other_state() {
        let mut state = TuiState::new("provider", "model");
        state.push_chat_message(TuiRole::User, "test");
        state.mode = InputMode::Normal;

        state.toggle_help();
        assert!(state.show_help);
        assert_eq!(state.mode, InputMode::Normal);
        assert_eq!(state.messages.len(), 1);

        state.toggle_help();
        assert!(!state.show_help);
    }

    /// Test pending approval state management
    #[test]
    fn pending_approval_state_lifecycle() {
        use clawclawclaw::tui::state::PendingApproval;

        let mut state = TuiState::new("provider", "model");
        assert!(state.pending_approval.is_none());

        state.pending_approval = Some(PendingApproval {
            request_id: "req-001".to_string(),
            tool_name: "shell".to_string(),
            arguments_summary: "rm -rf /tmp".to_string(),
        });
        assert!(state.pending_approval.is_some());
        assert_eq!(state.pending_approval.as_ref().unwrap().tool_name, "shell");

        // Simulating user approval clears the state
        state.pending_approval = None;
        assert!(state.pending_approval.is_none());
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
