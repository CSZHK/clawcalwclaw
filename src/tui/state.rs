//! TUI view-model state.
//!
//! This module intentionally stays UI-focused and does not import agent internals.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiStatus {
    Idle,
    Thinking,
    ToolRunning,
}

/// Status of a single tool call tracked in the structured tool panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallStatus {
    Running,
    Success(u64),
    Failed(u64),
}

/// A structured tool call entry for the tool panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallEntry {
    pub name: String,
    pub hint: String,
    pub status: ToolCallStatus,
}

/// Pending tool approval request displayed as a modal overlay.
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub request_id: String,
    pub tool_name: String,
    pub arguments_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiRole {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiChatMessage {
    pub role: TuiRole,
    pub content: String,
}

impl TuiChatMessage {
    pub fn new(role: TuiRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

#[derive(Debug)]
pub struct TuiState {
    pub messages: Vec<TuiChatMessage>,
    pub input_buffer: String,
    pub input_history: Vec<String>,
    pub scroll_offset: usize,
    pub should_quit: bool,
    pub mode: InputMode,
    pub progress_block: Option<String>,
    pub progress_line: Option<String>,
    pub status: TuiStatus,
    pub provider_id: String,
    pub model_id: String,
    pub awaiting_response: bool,
    pub streaming_assistant_idx: Option<usize>,

    // ── Structured tool tracking (Feature 1) ──
    pub tool_calls: Vec<ToolCallEntry>,

    // ── Token/cost tracking (Feature 2) ──
    pub session_input_tokens: u64,
    pub session_output_tokens: u64,
    pub session_cost_usd: f64,

    // ── Help overlay (Feature 3) ──
    pub show_help: bool,

    // ── Tool approval (Feature 4) ──
    pub pending_approval: Option<PendingApproval>,

    history_cursor: Option<usize>,
}

impl TuiState {
    pub fn new(provider_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            input_history: Vec::new(),
            scroll_offset: 0,
            should_quit: false,
            mode: InputMode::Editing,
            progress_block: None,
            progress_line: None,
            status: TuiStatus::Idle,
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            awaiting_response: false,
            streaming_assistant_idx: None,
            tool_calls: Vec::new(),
            session_input_tokens: 0,
            session_output_tokens: 0,
            session_cost_usd: 0.0,
            show_help: false,
            pending_approval: None,
            history_cursor: None,
        }
    }

    pub fn push_chat_message(&mut self, role: TuiRole, content: impl Into<String>) {
        self.messages.push(TuiChatMessage::new(role, content));
        // New messages should keep the viewport pinned to bottom by default.
        self.scroll_offset = 0;
    }

    pub fn start_streaming_assistant(&mut self) {
        if let Some(idx) = self.streaming_assistant_idx {
            if let Some(msg) = self.messages.get_mut(idx) {
                msg.content.clear();
                return;
            }
        }
        self.push_chat_message(TuiRole::Assistant, String::new());
        self.streaming_assistant_idx = Some(self.messages.len().saturating_sub(1));
    }

    pub fn append_stream_delta(&mut self, delta: &str) {
        if self.streaming_assistant_idx.is_none() {
            self.start_streaming_assistant();
        }
        if let Some(idx) = self.streaming_assistant_idx {
            if let Some(msg) = self.messages.get_mut(idx) {
                msg.content.push_str(delta);
            }
        }
    }

    pub fn finish_streaming_assistant(&mut self) {
        self.streaming_assistant_idx = None;
    }

    pub fn note_submitted_input(&mut self, content: &str) {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            self.history_cursor = None;
            return;
        }
        let should_push = self
            .input_history
            .last()
            .is_none_or(|last| last.as_str() != trimmed);
        if should_push {
            self.input_history.push(trimmed.to_string());
        }
        self.history_cursor = None;
    }

    pub fn history_prev(&mut self) -> Option<String> {
        if self.input_history.is_empty() {
            return None;
        }
        let next_idx = match self.history_cursor {
            None => self.input_history.len().saturating_sub(1),
            Some(idx) => idx.saturating_sub(1),
        };
        self.history_cursor = Some(next_idx);
        self.input_history.get(next_idx).cloned()
    }

    pub fn history_next(&mut self) -> Option<String> {
        let current = self.history_cursor?;
        if current + 1 >= self.input_history.len() {
            self.history_cursor = None;
            return Some(String::new());
        }
        let next_idx = current + 1;
        self.history_cursor = Some(next_idx);
        self.input_history.get(next_idx).cloned()
    }

    pub fn scroll_page_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines.max(1));
    }

    pub fn scroll_page_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines.max(1));
    }

    pub fn set_thinking(&mut self, line: Option<String>) {
        self.status = TuiStatus::Thinking;
        self.progress_line = line;
    }

    pub fn set_tool_running(&mut self, block: String) {
        self.status = TuiStatus::ToolRunning;
        self.progress_block = Some(block);
    }

    pub fn clear_progress(&mut self) {
        self.progress_line = None;
        self.progress_block = None;
    }

    pub fn set_idle(&mut self) {
        self.status = TuiStatus::Idle;
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            InputMode::Normal => InputMode::Editing,
            InputMode::Editing => InputMode::Normal,
        };
    }

    // ── Structured tool tracking ──

    pub fn add_tool_start(&mut self, name: String, hint: String) {
        self.tool_calls.push(ToolCallEntry {
            name,
            hint,
            status: ToolCallStatus::Running,
        });
    }

    pub fn complete_tool(&mut self, name: &str, success: bool, duration_secs: u64) {
        // Find the last running entry with this name (handles parallel tools).
        if let Some(entry) = self
            .tool_calls
            .iter_mut()
            .rev()
            .find(|e| e.name == name && e.status == ToolCallStatus::Running)
        {
            entry.status = if success {
                ToolCallStatus::Success(duration_secs)
            } else {
                ToolCallStatus::Failed(duration_secs)
            };
        }
    }

    pub fn clear_tool_calls(&mut self) {
        self.tool_calls.clear();
    }

    // ── Token/cost tracking ──

    pub fn accumulate_usage(
        &mut self,
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        cost_usd: Option<f64>,
    ) {
        if let Some(t) = input_tokens {
            self.session_input_tokens = self.session_input_tokens.saturating_add(t);
        }
        if let Some(t) = output_tokens {
            self.session_output_tokens = self.session_output_tokens.saturating_add(t);
        }
        if let Some(c) = cost_usd {
            self.session_cost_usd += c;
        }
    }

    // ── Help overlay ──

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}

#[cfg(test)]
mod tests {
    use super::{InputMode, ToolCallStatus, TuiRole, TuiState};

    #[test]
    fn push_message_appends_and_keeps_bottom_scroll() {
        let mut state = TuiState::new("provider", "model");
        state.scroll_offset = 8;
        state.push_chat_message(TuiRole::User, "hello");
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].content, "hello");
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn toggle_mode_switches_between_normal_and_editing() {
        let mut state = TuiState::new("provider", "model");
        assert_eq!(state.mode, InputMode::Editing);
        state.toggle_mode();
        assert_eq!(state.mode, InputMode::Normal);
        state.toggle_mode();
        assert_eq!(state.mode, InputMode::Editing);
    }

    #[test]
    fn scroll_offset_respects_saturating_bounds() {
        let mut state = TuiState::new("provider", "model");
        state.scroll_page_up(20);
        assert_eq!(state.scroll_offset, 20);
        state.scroll_page_down(7);
        assert_eq!(state.scroll_offset, 13);
        state.scroll_page_down(50);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn history_navigation_walks_back_and_forward() {
        let mut state = TuiState::new("provider", "model");
        state.note_submitted_input("first");
        state.note_submitted_input("second");
        state.note_submitted_input("third");

        assert_eq!(state.history_prev().as_deref(), Some("third"));
        assert_eq!(state.history_prev().as_deref(), Some("second"));
        assert_eq!(state.history_prev().as_deref(), Some("first"));
        assert_eq!(state.history_next().as_deref(), Some("second"));
        assert_eq!(state.history_next().as_deref(), Some("third"));
        assert_eq!(state.history_next().as_deref(), Some(""));
    }

    #[test]
    fn tool_call_lifecycle_start_complete_clear() {
        let mut state = TuiState::new("provider", "model");
        state.add_tool_start("shell".to_string(), "ls -la".to_string());
        assert_eq!(state.tool_calls.len(), 1);
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Running);

        state.complete_tool("shell", true, 2);
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Success(2));

        state.add_tool_start("file_read".to_string(), "src/main.rs".to_string());
        state.complete_tool("file_read", false, 1);
        assert_eq!(state.tool_calls[1].status, ToolCallStatus::Failed(1));

        state.clear_tool_calls();
        assert!(state.tool_calls.is_empty());
    }

    #[test]
    fn accumulate_usage_sums_correctly() {
        let mut state = TuiState::new("provider", "model");
        state.accumulate_usage(Some(100), Some(50), Some(0.01));
        state.accumulate_usage(Some(200), Some(100), Some(0.02));
        state.accumulate_usage(None, None, None);
        assert_eq!(state.session_input_tokens, 300);
        assert_eq!(state.session_output_tokens, 150);
        assert!((state.session_cost_usd - 0.03).abs() < f64::EPSILON);
    }

    #[test]
    fn toggle_help_switches_visibility() {
        let mut state = TuiState::new("provider", "model");
        assert!(!state.show_help);
        state.toggle_help();
        assert!(state.show_help);
        state.toggle_help();
        assert!(!state.show_help);
    }
}
