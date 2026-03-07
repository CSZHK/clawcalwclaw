//! TUI render tests using TestBackend (no real terminal needed).
//!
//! These tests verify widget rendering without touching crossterm.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

// Note: These tests require the tui-ratatui feature to be enabled.
// Run with: cargo test --features tui-ratatui tui_render

#[cfg(feature = "tui-ratatui")]
mod tui_render {
    use super::*;
    use clawclawclaw::tui::projections::{
        SubAgentPaneView, SubAgentProjectionItem, SubAgentViewStatus, TaskAuthorityKey,
        TaskBoardItem, TaskBoardStatus, TaskBoardView,
    };
    use clawclawclaw::tui::state::{
        ApprovalQueueItem, ApprovalQueueStatus, InputMode, TuiRole, TuiState,
    };
    use clawclawclaw::tui::widgets;

    fn create_test_terminal(size: (u16, u16)) -> Terminal<TestBackend> {
        let backend = TestBackend::new(size.0, size.1);
        Terminal::new(backend).expect("failed to create test terminal")
    }

    fn render_content(terminal: &mut Terminal<TestBackend>, state: &TuiState) -> String {
        terminal
            .draw(|frame| {
                widgets::render(frame, state);
            })
            .expect("draw failed");

        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    fn approval_item(request_id: &str, status: ApprovalQueueStatus) -> ApprovalQueueItem {
        ApprovalQueueItem {
            request_id: request_id.to_string(),
            tool_name: "shell".to_string(),
            arguments_summary: "rm -rf /tmp/test".to_string(),
            requested_at: "10:00:00".to_string(),
            status,
            status_message: None,
        }
    }

    #[test]
    fn renders_initial_state_with_system_message() {
        let mut terminal = create_test_terminal((80, 24));
        let state = TuiState::new("test-provider", "test-model");

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        let buffer = terminal.backend().buffer();
        // Verify the buffer contains expected content
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(
            content.contains("test-provider") || content.contains("test-model"),
            "Provider/model should appear in status"
        );
    }

    #[test]
    fn renders_user_message_in_chat() {
        let mut terminal = create_test_terminal((80, 24));
        let mut state = TuiState::new("provider", "model");
        state.push_chat_message(TuiRole::User, "Hello, world!");

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(
            content.contains("Hello"),
            "User message should appear in chat"
        );
    }

    #[test]
    fn renders_progress_block_when_set() {
        let mut terminal = create_test_terminal((80, 24));
        let mut state = TuiState::new("provider", "model");
        state.set_tool_running("Running shell: ls".to_string());

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(
            content.contains("shell") || content.contains("Running"),
            "Progress block should appear when tool running"
        );
    }

    #[test]
    fn input_mode_changes_cursor_visibility() {
        let mut terminal = create_test_terminal((80, 24));
        let mut state = TuiState::new("provider", "model");

        // Editing mode should show cursor
        state.mode = InputMode::Editing;
        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        // Normal mode should not show cursor in input area
        state.mode = InputMode::Normal;
        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        // Cursor position behavior differs between modes
        // (verification would require capturing cursor state)
    }

    #[test]
    fn renders_structured_tool_panel_with_entries() {
        let mut terminal = create_test_terminal((80, 24));
        let mut state = TuiState::new("provider", "model");
        state.add_tool_start(0, "shell".to_string(), "ls -la".to_string());
        state.add_tool_start(1, "file_read".to_string(), "src/main.rs".to_string());
        state.complete_tool(1, true, 2);

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Tool panel title should be visible
        assert!(
            content.contains("Tools"),
            "Tools panel should appear when structured entries exist"
        );
        // Tool names should appear
        assert!(
            content.contains("shell"),
            "Tool name 'shell' should appear in panel"
        );
        assert!(
            content.contains("file_read"),
            "Tool name 'file_read' should appear in panel"
        );
    }

    #[test]
    fn renders_help_overlay_when_active() {
        let mut terminal = create_test_terminal((80, 24));
        let mut state = TuiState::new("provider", "model");
        state.show_help = true;

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Help overlay should contain keybinding references
        assert!(
            content.contains("Esc") || content.contains("Help") || content.contains("Keybindings"),
            "Help overlay should be visible when show_help is true"
        );
    }

    #[test]
    fn renders_approval_modal_when_pending() {
        let mut terminal = create_test_terminal((80, 24));
        let mut state = TuiState::new("provider", "model");
        state.enqueue_approval(approval_item("req-123", ApprovalQueueStatus::Pending));

        let content = render_content(&mut terminal, &state);

        assert!(
            content.contains("Approval") || content.contains("shell"),
            "Approval modal should show tool name"
        );
    }

    #[test]
    fn renders_terminal_approval_states_with_default_copy() {
        let terminal_states = [
            (
                ApprovalQueueStatus::Approved,
                "Approved and forwarded to runtime",
            ),
            (ApprovalQueueStatus::Denied, "Denied and kept fail-closed"),
            (
                ApprovalQueueStatus::Failed,
                "Approval bridge failed; runtime remains fail-closed",
            ),
            (
                ApprovalQueueStatus::Expired,
                "Approval expired; runtime remains fail-closed",
            ),
        ];

        for (status, status_copy) in terminal_states {
            let mut terminal = create_test_terminal((120, 30));
            let mut state = TuiState::new("provider", "model");
            state.enqueue_approval(approval_item("req-terminal", status));

            let content = render_content(&mut terminal, &state);

            assert!(content.contains(status_copy), "missing copy for {status:?}");
            assert!(
                content.contains("dismiss this approval item"),
                "terminal approval should be dismissable for {status:?}"
            );
            assert!(
                !content.contains("Allow this tool call?"),
                "terminal approval should not show pending prompt for {status:?}"
            );
        }
    }

    #[test]
    fn renders_token_cost_in_status_bar() {
        let mut terminal = create_test_terminal((80, 24));
        let mut state = TuiState::new("test-provider", "test-model");
        state.accumulate_usage(Some(12345), Some(6789), Some(0.04));

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(
            content.contains("tokens"),
            "Status bar should display token count"
        );
        assert!(content.contains("$0.04"), "Status bar should display cost");
    }

    #[test]
    fn scroll_offset_affects_visible_messages() {
        let mut terminal = create_test_terminal((80, 10)); // Small height
        let mut state = TuiState::new("provider", "model");

        // Add many messages
        for i in 0..20 {
            state.push_chat_message(TuiRole::User, format!("Message number {}", i));
        }
        state.scroll_page_up(5);

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        // After scrolling, early messages should be visible
        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // The scroll_offset should affect what's visible
        assert!(state.scroll_offset > 0);
        let _ = content; // Content analysis would need more sophisticated parsing
    }

    #[test]
    fn renders_task_board_sidebar_when_projection_present() {
        let mut terminal = create_test_terminal((100, 28));
        let mut state = TuiState::new("provider", "model");
        state.set_task_board_view(Some(TaskBoardView {
            durable_items: vec![TaskBoardItem {
                authority: TaskAuthorityKey::GoalStep {
                    goal_id: "g1".to_string(),
                    step_id: "s1".to_string(),
                },
                title: "Design bridge".to_string(),
                status: TaskBoardStatus::InProgress,
                priority_label: Some("High".to_string()),
                group_label: "Workbench".to_string(),
                detail_summary: None,
            }],
            session_items: Vec::new(),
            merged_items: vec![TaskBoardItem {
                authority: TaskAuthorityKey::GoalStep {
                    goal_id: "g1".to_string(),
                    step_id: "s1".to_string(),
                },
                title: "Design bridge".to_string(),
                status: TaskBoardStatus::InProgress,
                priority_label: Some("High".to_string()),
                group_label: "Workbench".to_string(),
                detail_summary: None,
            }],
            refreshed_at: "10:00:00".to_string(),
            error_summary: None,
        }));

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        let content = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("Task Board"));
        assert!(content.contains("Design bridge"));
    }

    #[test]
    fn hides_empty_task_board_projection_until_content_exists() {
        let mut terminal = create_test_terminal((100, 28));
        let mut state = TuiState::new("provider", "model");
        state.push_chat_message(TuiRole::User, "still chatting");
        state.set_task_board_view(Some(TaskBoardView {
            refreshed_at: "10:00:00".to_string(),
            ..TaskBoardView::default()
        }));

        let content = render_content(&mut terminal, &state);

        assert!(content.contains("still chatting"));
        assert!(!content.contains("Task Board"));
        assert!(!content.contains("No goals or session tasks yet."));
    }

    #[test]
    fn renders_session_only_task_board_group() {
        let session_item = TaskBoardItem {
            authority: TaskAuthorityKey::SessionTask { task_id: 7 },
            title: "Review checklist".to_string(),
            status: TaskBoardStatus::Completed,
            priority_label: None,
            group_label: "Session Plan".to_string(),
            detail_summary: Some("done".to_string()),
        };

        let mut terminal = create_test_terminal((120, 30));
        let mut state = TuiState::new("provider", "model");
        state.set_task_board_view(Some(TaskBoardView {
            session_items: vec![session_item.clone()],
            merged_items: vec![session_item],
            refreshed_at: "10:00:00".to_string(),
            error_summary: None,
            ..TaskBoardView::default()
        }));

        let content = render_content(&mut terminal, &state);

        assert!(content.contains("Task Board"));
        assert!(content.contains("Session Plan"));
        assert!(content.contains("Review checklist"));
        assert!(!content.contains("Durable Goals"));
    }

    #[test]
    fn renders_dual_authority_task_board_entries_with_same_title() {
        let shared_title = "Design bridge";
        let durable_item = TaskBoardItem {
            authority: TaskAuthorityKey::GoalStep {
                goal_id: "g1".to_string(),
                step_id: "s1".to_string(),
            },
            title: shared_title.to_string(),
            status: TaskBoardStatus::InProgress,
            priority_label: Some("High".to_string()),
            group_label: "Durable Goals".to_string(),
            detail_summary: Some("waiting on review".to_string()),
        };
        let session_item = TaskBoardItem {
            authority: TaskAuthorityKey::SessionTask { task_id: 2 },
            title: shared_title.to_string(),
            status: TaskBoardStatus::Pending,
            priority_label: None,
            group_label: "Session Plan".to_string(),
            detail_summary: Some("mirrors active task".to_string()),
        };

        let mut terminal = create_test_terminal((120, 30));
        let mut state = TuiState::new("provider", "model");
        state.set_task_board_view(Some(TaskBoardView {
            durable_items: vec![durable_item.clone()],
            session_items: vec![session_item.clone()],
            merged_items: vec![durable_item, session_item],
            refreshed_at: "10:00:00".to_string(),
            error_summary: None,
        }));

        let content = render_content(&mut terminal, &state);

        assert!(content.contains("Durable Goals"));
        assert!(content.contains("Session Plan"));
        assert_eq!(content.matches(shared_title).count(), 2);
    }

    #[test]
    fn keeps_task_board_readable_on_small_terminal() {
        let session_item = TaskBoardItem {
            authority: TaskAuthorityKey::SessionTask { task_id: 9 },
            title: "Ship".to_string(),
            status: TaskBoardStatus::InProgress,
            priority_label: None,
            group_label: "Session Plan".to_string(),
            detail_summary: None,
        };

        let mut terminal = create_test_terminal((60, 12));
        let mut state = TuiState::new("provider", "model");
        state.set_task_board_view(Some(TaskBoardView {
            session_items: vec![session_item.clone()],
            merged_items: vec![session_item],
            refreshed_at: "10:00:00".to_string(),
            error_summary: None,
            ..TaskBoardView::default()
        }));

        let content = render_content(&mut terminal, &state);

        assert!(content.contains("Task Board"));
        assert!(content.contains("Ship"));
    }

    #[test]
    fn renders_subagent_sidebar_when_projection_present() {
        let mut terminal = create_test_terminal((100, 28));
        let mut state = TuiState::new("provider", "model");
        state.set_subagent_pane_view(Some(SubAgentPaneView {
            items: vec![SubAgentProjectionItem {
                session_id: "s1".to_string(),
                agent_name: "researcher".to_string(),
                status: SubAgentViewStatus::Running,
                task_summary: "Trace workbench bridge".to_string(),
                started_at: "10:00:00".to_string(),
                completed_at: None,
                last_event_summary: Some("Running shell".to_string()),
                last_tool_name: Some("shell".to_string()),
                input_tokens: None,
                output_tokens: None,
                error_summary: None,
            }],
            refreshed_at: "10:00:01".to_string(),
        }));

        terminal
            .draw(|frame| {
                widgets::render(frame, &state);
            })
            .expect("draw failed");

        let content = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("Sub-Agents"));
        assert!(content.contains("researcher"));
    }

    #[test]
    fn renders_terminal_subagent_states() {
        let mut terminal = create_test_terminal((120, 30));
        let mut state = TuiState::new("provider", "model");
        state.set_subagent_pane_view(Some(SubAgentPaneView {
            items: vec![
                SubAgentProjectionItem {
                    session_id: "done".to_string(),
                    agent_name: "completed-worker".to_string(),
                    status: SubAgentViewStatus::Completed,
                    task_summary: "Close checklist".to_string(),
                    started_at: "10:00:00".to_string(),
                    completed_at: Some("10:00:05".to_string()),
                    last_event_summary: Some("Finished file_read (2.0s)".to_string()),
                    last_tool_name: Some("file_read".to_string()),
                    input_tokens: Some(12),
                    output_tokens: Some(34),
                    error_summary: None,
                },
                SubAgentProjectionItem {
                    session_id: "fail".to_string(),
                    agent_name: "failed-worker".to_string(),
                    status: SubAgentViewStatus::Failed,
                    task_summary: "Trace approval bridge".to_string(),
                    started_at: "10:01:00".to_string(),
                    completed_at: Some("10:01:03".to_string()),
                    last_event_summary: Some("Failed shell (1.0s)".to_string()),
                    last_tool_name: Some("shell".to_string()),
                    input_tokens: None,
                    output_tokens: None,
                    error_summary: Some("runtime exploded".to_string()),
                },
                SubAgentProjectionItem {
                    session_id: "kill".to_string(),
                    agent_name: "killed-worker".to_string(),
                    status: SubAgentViewStatus::Killed,
                    task_summary: "Abort stale branch".to_string(),
                    started_at: "10:02:00".to_string(),
                    completed_at: Some("10:02:01".to_string()),
                    last_event_summary: None,
                    last_tool_name: None,
                    input_tokens: None,
                    output_tokens: None,
                    error_summary: None,
                },
            ],
            refreshed_at: "10:02:02".to_string(),
        }));

        let content = render_content(&mut terminal, &state);

        assert!(content.contains("Sub-Agents"));
        assert!(content.contains("[✓] completed-worker"));
        assert!(content.contains("[!] failed-worker"));
        assert!(content.contains("[x] killed-worker"));
        assert!(content.contains("runtime exploded"));
    }

    #[test]
    fn omits_subagent_telemetry_labels_when_data_is_missing() {
        let mut terminal = create_test_terminal((120, 30));
        let mut state = TuiState::new("provider", "model");
        state.set_subagent_pane_view(Some(SubAgentPaneView {
            items: vec![SubAgentProjectionItem {
                session_id: "gap".to_string(),
                agent_name: "quiet-worker".to_string(),
                status: SubAgentViewStatus::Completed,
                task_summary: "Wait for telemetry".to_string(),
                started_at: "10:03:00".to_string(),
                completed_at: Some("10:03:05".to_string()),
                last_event_summary: None,
                last_tool_name: None,
                input_tokens: None,
                output_tokens: None,
                error_summary: None,
            }],
            refreshed_at: "10:03:06".to_string(),
        }));

        let content = render_content(&mut terminal, &state);

        assert!(content.contains("quiet-worker"));
        assert!(content.contains("Wait for telemetry"));
        assert!(!content.contains("Event:"));
        assert!(!content.contains("Tool:"));
        assert!(!content.contains("Error:"));
    }

    #[test]
    fn help_overlay_coexists_with_visible_workbench_panes() {
        let mut terminal = create_test_terminal((200, 40));
        let mut state = TuiState::new("provider", "model");
        state.show_help = true;
        state.set_task_board_view(Some(TaskBoardView {
            session_items: vec![TaskBoardItem {
                authority: TaskAuthorityKey::SessionTask { task_id: 3 },
                title: "Review workbench overlay".to_string(),
                status: TaskBoardStatus::InProgress,
                priority_label: None,
                group_label: "Session Plan".to_string(),
                detail_summary: None,
            }],
            merged_items: vec![TaskBoardItem {
                authority: TaskAuthorityKey::SessionTask { task_id: 3 },
                title: "Review workbench overlay".to_string(),
                status: TaskBoardStatus::InProgress,
                priority_label: None,
                group_label: "Session Plan".to_string(),
                detail_summary: None,
            }],
            refreshed_at: "10:04:00".to_string(),
            error_summary: None,
            ..TaskBoardView::default()
        }));
        state.set_subagent_pane_view(Some(SubAgentPaneView {
            items: vec![SubAgentProjectionItem {
                session_id: "s-help".to_string(),
                agent_name: "overlay-worker".to_string(),
                status: SubAgentViewStatus::Running,
                task_summary: "Verify sidebar visibility".to_string(),
                started_at: "10:04:01".to_string(),
                completed_at: None,
                last_event_summary: Some("Running shell".to_string()),
                last_tool_name: Some("shell".to_string()),
                input_tokens: None,
                output_tokens: None,
                error_summary: None,
            }],
            refreshed_at: "10:04:02".to_string(),
        }));

        let content = render_content(&mut terminal, &state);

        assert!(content.contains("Help"));
        assert!(content.contains("Task Board"));
        assert!(content.contains("Sub-Agents"));
        assert!(content.contains("overlay-worker"));
    }
}
