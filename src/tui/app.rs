//! TUI application runtime and event loop.
//!
//! Initialization order:
//! 1) panic hook
//! 2) signal handlers
//! 3) raw mode + alternate screen
//! 4) async event loop (`EventStream` + agent delta channel)

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::future::pending;

use anyhow::Result;
use futures_util::StreamExt;
use ratatui::crossterm::event::{
    Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use tokio_util::sync::CancellationToken;

use crate::agent::loop_::{
    build_shell_policy_instructions, build_tool_instructions, create_cost_enforcement_context,
    lookup_model_pricing, run_tool_call_loop_with_non_cli_approval_context, scope_agent_events,
    scope_cost_enforcement_context, AgentEvent, NonCliApprovalContext, NonCliApprovalPrompt,
    SafetyHeartbeatConfig,
};
use crate::config::{resolve_default_model_id, Config, ProgressMode};
use crate::goals::engine::GoalEngine;
use crate::memory::{self, Memory};
use crate::observability::{self, Observer};
use crate::providers::{self, ChatMessage, Provider};
use crate::runtime;
use crate::security::SecurityPolicy;
use crate::tools::{self, SubAgentRegistry, TaskPlanTool, Tool};
use crate::util::truncate_with_ellipsis;

use super::events::{translate_delta, TuiEvent};
use super::projections::{
    build_subagent_observer_factory, build_subagent_pane_view, build_task_board_view,
    SubagentTelemetryCache, SubagentTelemetryEvent,
};
use super::state::{ApprovalQueueItem, ApprovalQueueStatus, InputMode, TuiRole, TuiState};
use super::terminal::{install_panic_hook, install_signal_handlers};
use super::widgets;

const DELTA_CHANNEL_BUFFER: usize = 256;
const AGENT_EVENT_CHANNEL_BUFFER: usize = 256;
const DOUBLE_CTRL_C_WINDOW: Duration = Duration::from_millis(300);
const WORKBENCH_REFRESH_INTERVAL: Duration = Duration::from_millis(750);
const APPROVAL_PREVIEW_MAX_LEN: usize = 240;
const TUI_APPROVAL_SENDER: &str = "tui-user";
const TUI_APPROVAL_CHANNEL: &str = "tui";
const TUI_APPROVAL_REPLY_TARGET: &str = "tui";

#[derive(Debug)]
struct AgentTaskResult {
    request_id: u64,
    history: Vec<ChatMessage>,
    output: std::result::Result<String, String>,
}

#[derive(Clone)]
struct WorkbenchReadHandles {
    goal_engine: GoalEngine,
    task_plan: Arc<TaskPlanTool>,
    subagent_registry: Option<Arc<SubAgentRegistry>>,
}

struct TuiRuntimeContext {
    config: Config,
    provider_name: String,
    model_name: String,
    provider: Arc<dyn Provider>,
    observer: Arc<dyn Observer>,
    tools_registry: Arc<Vec<Box<dyn Tool>>>,
    multimodal: crate::config::MultimodalConfig,
    max_tool_iterations: usize,
    hooks: Option<Arc<crate::hooks::HookRunner>>,
    safety_heartbeat: Option<SafetyHeartbeatConfig>,
    cost_enforcement: Option<crate::agent::loop_::CostEnforcementContext>,
    approval_manager: Arc<crate::approval::ApprovalManager>,
    temperature: f64,
    history: Vec<ChatMessage>,
    canary_tokens_enabled: bool,
    workbench: Option<WorkbenchReadHandles>,
    subagent_telemetry_rx: Option<tokio::sync::mpsc::UnboundedReceiver<SubagentTelemetryEvent>>,
}

pub async fn run(config: &Config) -> Result<()> {
    install_panic_hook();
    let session_cancel = CancellationToken::new();
    install_signal_handlers(session_cancel.child_token()).await;

    ratatui::crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    ratatui::crossterm::execute!(
        stdout,
        ratatui::crossterm::terminal::EnterAlternateScreen,
        ratatui::crossterm::cursor::Hide
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let app_result = run_loop(&mut terminal, config, session_cancel).await;

    let _ = ratatui::crossterm::execute!(
        terminal.backend_mut(),
        ratatui::crossterm::terminal::LeaveAlternateScreen,
        ratatui::crossterm::cursor::Show
    );
    let _ = ratatui::crossterm::terminal::disable_raw_mode();

    app_result
}

async fn run_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    config: &Config,
    session_cancel: CancellationToken,
) -> Result<()> {
    let mut runtime_ctx = bootstrap_runtime(config).await?;
    let mut state = TuiState::new(
        runtime_ctx.provider_name.clone(),
        runtime_ctx.model_name.clone(),
    );
    state.push_chat_message(
        TuiRole::System,
        "clawclawclaw TUI ready. Press i to edit, Enter to send, Ctrl+C to cancel, q to quit.",
    );
    if is_provider_api_key_missing(&runtime_ctx) {
        state.push_chat_message(
            TuiRole::System,
            missing_api_key_guidance(&runtime_ctx.provider_name),
        );
    }
    let mut subagent_telemetry = SubagentTelemetryCache::default();
    refresh_workbench_views(&runtime_ctx, &mut state, &subagent_telemetry).await;

    let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel::<String>(DELTA_CHANNEL_BUFFER);
    let (result_tx, mut result_rx) = tokio::sync::mpsc::unbounded_channel::<AgentTaskResult>();
    let (agent_event_tx, mut agent_event_rx) =
        tokio::sync::mpsc::channel::<AgentEvent>(AGENT_EVENT_CHANNEL_BUFFER);
    let (approval_prompt_tx, mut approval_prompt_rx) =
        tokio::sync::mpsc::unbounded_channel::<NonCliApprovalPrompt>();
    let mut event_stream = EventStream::new();
    let mut active_request_cancel: Option<CancellationToken> = None;
    let mut active_request_id: Option<u64> = None;
    let mut next_request_id = 1_u64;
    let mut last_ctrl_c_at: Option<Instant> = None;
    let mut workbench_refresh = tokio::time::interval(WORKBENCH_REFRESH_INTERVAL);
    workbench_refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        terminal.draw(|frame| {
            if let Some(cursor) = widgets::render(frame, &state) {
                frame.set_cursor_position((cursor.x, cursor.y));
            }
        })?;

        let mut needs_workbench_refresh = false;

        tokio::select! {
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(event)) => {
                        let is_resize = matches!(&event, Event::Resize(_, _));
                        let approval_visible = !state.approval_queue.is_empty();
                        handle_terminal_event(
                            event,
                            &mut state,
                            &mut runtime_ctx,
                            &delta_tx,
                            &result_tx,
                            &agent_event_tx,
                            &approval_prompt_tx,
                            &mut active_request_cancel,
                            &mut active_request_id,
                            &mut next_request_id,
                            &mut last_ctrl_c_at,
                        )
                        .await?;
                        needs_workbench_refresh = approval_visible || !state.approval_queue.is_empty();
                        if is_resize {
                            terminal.autoresize()?;
                        }
                    }
                    Some(Err(error)) => {
                        state.push_chat_message(TuiRole::Error, format!("Input error: {error}"));
                    }
                    None => {
                        state.should_quit = true;
                    }
                }
            }
            Some(delta) = delta_rx.recv() => {
                handle_tui_event(translate_delta(delta), &mut state);
            }
            Some(agent_event) = agent_event_rx.recv() => {
                handle_agent_event(agent_event, &mut state, &runtime_ctx);
                needs_workbench_refresh = true;
            }
            Some(prompt) = approval_prompt_rx.recv() => {
                handle_approval_prompt(prompt, &mut state);
                needs_workbench_refresh = true;
            }
            Some(task_result) = result_rx.recv() => {
                if active_request_id != Some(task_result.request_id) {
                    continue;
                }
                active_request_cancel = None;
                active_request_id = None;
                state.awaiting_response = false;
                state.set_idle();
                state.clear_progress();
                state.clear_tool_calls();

                runtime_ctx.history = task_result.history;
                match task_result.output {
                    Ok(output) => {
                        finalize_assistant_output(&mut state, output);
                    }
                    Err(error) => {
                        state.finish_streaming_assistant();
                        let friendly = friendly_error_message(&error, &runtime_ctx.provider_name);
                        state.push_chat_message(TuiRole::Error, friendly);
                    }
                }
                needs_workbench_refresh = true;
            }
            subagent_event = recv_subagent_telemetry(&mut runtime_ctx) => {
                match subagent_event {
                    Some(event) => {
                        subagent_telemetry.record(event);
                        needs_workbench_refresh = true;
                    }
                    None => {
                        runtime_ctx.subagent_telemetry_rx = None;
                    }
                }
            }
            _ = workbench_refresh.tick() => {
                needs_workbench_refresh = true;
            }
            _ = session_cancel.cancelled() => {
                state.should_quit = true;
            }
        }

        reconcile_approval_queue(&runtime_ctx, &mut state);
        if needs_workbench_refresh {
            refresh_workbench_views(&runtime_ctx, &mut state, &subagent_telemetry).await;
        }

        if state.should_quit {
            if let Some(cancel) = active_request_cancel.take() {
                cancel.cancel();
            }
            break;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_terminal_event(
    event: Event,
    state: &mut TuiState,
    runtime_ctx: &mut TuiRuntimeContext,
    delta_tx: &tokio::sync::mpsc::Sender<String>,
    result_tx: &tokio::sync::mpsc::UnboundedSender<AgentTaskResult>,
    agent_event_tx: &tokio::sync::mpsc::Sender<AgentEvent>,
    approval_prompt_tx: &tokio::sync::mpsc::UnboundedSender<NonCliApprovalPrompt>,
    active_request_cancel: &mut Option<CancellationToken>,
    active_request_id: &mut Option<u64>,
    next_request_id: &mut u64,
    last_ctrl_c_at: &mut Option<Instant>,
) -> Result<()> {
    match event {
        Event::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            let handled = handle_key_event(
                key,
                state,
                runtime_ctx,
                delta_tx,
                result_tx,
                agent_event_tx,
                approval_prompt_tx,
                active_request_cancel,
                active_request_id,
                next_request_id,
                last_ctrl_c_at,
            )
            .await?;
            if !handled && state.mode == InputMode::Editing {
                handle_editing_text_input(key, state);
            }
        }
        Event::Resize(width, height) => {
            handle_tui_event(TuiEvent::Resize(width, height), state);
        }
        Event::Paste(payload) => {
            if state.mode == InputMode::Editing {
                super::widgets::input::append_sanitized_input(&mut state.input_buffer, &payload);
            }
        }
        _ => {}
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_key_event(
    key: KeyEvent,
    state: &mut TuiState,
    runtime_ctx: &mut TuiRuntimeContext,
    delta_tx: &tokio::sync::mpsc::Sender<String>,
    result_tx: &tokio::sync::mpsc::UnboundedSender<AgentTaskResult>,
    agent_event_tx: &tokio::sync::mpsc::Sender<AgentEvent>,
    approval_prompt_tx: &tokio::sync::mpsc::UnboundedSender<NonCliApprovalPrompt>,
    active_request_cancel: &mut Option<CancellationToken>,
    active_request_id: &mut Option<u64>,
    next_request_id: &mut u64,
    last_ctrl_c_at: &mut Option<Instant>,
) -> Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d') {
        trigger_quit(state, active_request_cancel, active_request_id);
        return Ok(true);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        let now = Instant::now();
        if last_ctrl_c_at
            .as_ref()
            .is_some_and(|last| now.duration_since(*last) <= DOUBLE_CTRL_C_WINDOW)
        {
            trigger_quit(state, active_request_cancel, active_request_id);
            return Ok(true);
        }
        *last_ctrl_c_at = Some(now);
        if let Some(cancel) = active_request_cancel.take() {
            cancel.cancel();
            *active_request_id = None;
            state.awaiting_response = false;
            state.set_idle();
            state.clear_progress();
            state.finish_streaming_assistant();
            state.push_chat_message(TuiRole::System, "Cancelled current request.");
        }
        return Ok(true);
    }

    // ── Approval modal intercepts keys before normal mode handling ──
    if let Some(active_request_id) = state.active_approval().map(|item| item.request_id.clone()) {
        let active_status = state.active_approval().map(|item| item.status);
        match key.code {
            KeyCode::Char('y' | 'Y') if active_status == Some(ApprovalQueueStatus::Pending) => {
                resolve_approval_request(
                    state,
                    &runtime_ctx.approval_manager,
                    &active_request_id,
                    crate::approval::ApprovalResponse::Yes,
                );
                return Ok(true);
            }
            KeyCode::Char('n' | 'N') if active_status == Some(ApprovalQueueStatus::Pending) => {
                resolve_approval_request(
                    state,
                    &runtime_ctx.approval_manager,
                    &active_request_id,
                    crate::approval::ApprovalResponse::No,
                );
                return Ok(true);
            }
            KeyCode::Esc | KeyCode::Enter
                if active_status.is_some_and(ApprovalQueueStatus::is_terminal) =>
            {
                state.dismiss_approval(&active_request_id);
                return Ok(true);
            }
            _ => return Ok(true),
        }
    }

    // ── Help overlay: close on any key ──
    if state.show_help {
        state.show_help = false;
        return Ok(true);
    }

    match state.mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => {
                trigger_quit(state, active_request_cancel, active_request_id);
                Ok(true)
            }
            KeyCode::Char('i') => {
                state.mode = InputMode::Editing;
                Ok(true)
            }
            KeyCode::Char('?') => {
                state.toggle_help();
                Ok(true)
            }
            KeyCode::PageUp => {
                state.scroll_page_up(12);
                Ok(true)
            }
            KeyCode::PageDown => {
                state.scroll_page_down(12);
                Ok(true)
            }
            _ => Ok(false),
        },
        InputMode::Editing => match key.code {
            KeyCode::Esc => {
                state.mode = InputMode::Normal;
                Ok(true)
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                super::widgets::input::append_sanitized_input(&mut state.input_buffer, "\n");
                Ok(true)
            }
            KeyCode::Enter => {
                if !state.awaiting_response {
                    submit_user_message(
                        state,
                        runtime_ctx,
                        delta_tx,
                        result_tx,
                        agent_event_tx,
                        approval_prompt_tx,
                        active_request_cancel,
                        active_request_id,
                        next_request_id,
                    )
                    .await?;
                }
                Ok(true)
            }
            KeyCode::Up => {
                if let Some(prev) = state.history_prev() {
                    state.input_buffer = prev;
                }
                Ok(true)
            }
            KeyCode::Down => {
                if let Some(next) = state.history_next() {
                    state.input_buffer = next;
                }
                Ok(true)
            }
            KeyCode::PageUp => {
                state.scroll_page_up(12);
                Ok(true)
            }
            KeyCode::PageDown => {
                state.scroll_page_down(12);
                Ok(true)
            }
            KeyCode::Backspace => {
                state.input_buffer.pop();
                Ok(true)
            }
            KeyCode::Tab => {
                super::widgets::input::append_sanitized_input(&mut state.input_buffer, "\t");
                Ok(true)
            }
            _ => Ok(false),
        },
    }
}

fn handle_editing_text_input(key: KeyEvent, state: &mut TuiState) {
    if state.mode != InputMode::Editing {
        return;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return;
    }
    if let KeyCode::Char(ch) = key.code {
        let mut utf8_buf = [0_u8; 4];
        let as_str = ch.encode_utf8(&mut utf8_buf);
        super::widgets::input::append_sanitized_input(&mut state.input_buffer, as_str);
    }
}

fn handle_tui_event(event: TuiEvent, state: &mut TuiState) {
    match event {
        TuiEvent::Delta { text } => {
            state.append_stream_delta(&text);
        }
        TuiEvent::Clear => {
            state.start_streaming_assistant();
        }
        TuiEvent::ProgressLine { text } => {
            state.set_thinking(Some(text));
        }
        TuiEvent::ProgressBlock { content } => {
            state.set_tool_running(content);
        }
        TuiEvent::Resize(width, height) => {
            let _ = (width, height);
        }
        TuiEvent::UserMessage { .. } | TuiEvent::Cancel | TuiEvent::Quit | TuiEvent::Key(_) => {}
    }
}

fn handle_agent_event(event: AgentEvent, state: &mut TuiState, runtime_ctx: &TuiRuntimeContext) {
    match event {
        AgentEvent::ToolStart {
            progress_id,
            name,
            hint,
        } => {
            state.add_tool_start(progress_id, name, hint);
        }
        AgentEvent::ToolComplete {
            progress_id,
            success,
            duration_secs,
        } => {
            state.complete_tool(progress_id, success, duration_secs);
        }
        AgentEvent::Usage {
            provider,
            model,
            input_tokens,
            output_tokens,
            cost_usd,
        } => {
            let estimated_cost = cost_usd.or_else(|| {
                estimate_usage_cost(
                    &runtime_ctx.config.cost.prices,
                    &provider,
                    &model,
                    input_tokens,
                    output_tokens,
                )
            });
            state.accumulate_usage(input_tokens, output_tokens, estimated_cost);
        }
    }
}

fn handle_approval_prompt(prompt: NonCliApprovalPrompt, state: &mut TuiState) {
    let args_summary = truncate_json_for_display(&prompt.arguments, APPROVAL_PREVIEW_MAX_LEN);
    state.enqueue_approval(ApprovalQueueItem {
        request_id: prompt.request_id,
        tool_name: prompt.tool_name,
        arguments_summary: args_summary,
        requested_at: chrono::Local::now().format("%H:%M:%S").to_string(),
        status: ApprovalQueueStatus::Pending,
        status_message: None,
    });
}

fn resolve_approval_request(
    state: &mut TuiState,
    approval_manager: &crate::approval::ApprovalManager,
    request_id: &str,
    decision: crate::approval::ApprovalResponse,
) {
    let resolution = match decision {
        crate::approval::ApprovalResponse::Yes => approval_manager
            .confirm_non_cli_pending_request(
                request_id,
                TUI_APPROVAL_SENDER,
                TUI_APPROVAL_CHANNEL,
                TUI_APPROVAL_REPLY_TARGET,
            )
            .map(|_| (ApprovalQueueStatus::Approved, "Approved in TUI".to_string())),
        crate::approval::ApprovalResponse::No => approval_manager
            .reject_non_cli_pending_request(
                request_id,
                TUI_APPROVAL_SENDER,
                TUI_APPROVAL_CHANNEL,
                TUI_APPROVAL_REPLY_TARGET,
            )
            .map(|_| (ApprovalQueueStatus::Denied, "Denied in TUI".to_string())),
        _ => Err(crate::approval::PendingApprovalError::NotFound),
    };

    let (status, message) = match resolution {
        Ok(value) => value,
        Err(crate::approval::PendingApprovalError::Expired) => (
            ApprovalQueueStatus::Expired,
            "Approval expired before resolution".to_string(),
        ),
        Err(crate::approval::PendingApprovalError::RequesterMismatch) => (
            ApprovalQueueStatus::Failed,
            "Approval requester mismatch; runtime stayed fail-closed".to_string(),
        ),
        Err(crate::approval::PendingApprovalError::NotFound) => (
            ApprovalQueueStatus::Failed,
            "Approval request no longer exists; runtime stayed fail-closed".to_string(),
        ),
    };

    approval_manager.record_non_cli_pending_resolution(request_id, decision);
    let _ = state.update_approval_status(request_id, status, Some(message));
}

fn estimate_usage_cost(
    prices: &std::collections::HashMap<String, crate::config::schema::ModelPricing>,
    provider: &str,
    model: &str,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
) -> Option<f64> {
    if input_tokens.is_none() && output_tokens.is_none() {
        return None;
    }

    let (input_price, output_price) = lookup_model_pricing(prices, provider, model);
    let input_cost = (input_tokens.unwrap_or(0) as f64 / 1_000_000.0) * input_price.max(0.0);
    let output_cost = (output_tokens.unwrap_or(0) as f64 / 1_000_000.0) * output_price.max(0.0);
    Some(input_cost + output_cost)
}

/// Truncate a JSON value to a human-readable summary for the approval modal.
fn truncate_json_for_display(value: &serde_json::Value, max_len: usize) -> String {
    let redacted = redact_json_for_preview(value, 0);
    truncate_with_ellipsis(&redacted.to_string(), max_len)
}

fn redact_json_for_preview(value: &serde_json::Value, depth: usize) -> serde_json::Value {
    const MAX_DEPTH: usize = 3;

    if depth >= MAX_DEPTH {
        return serde_json::Value::String("[truncated]".to_string());
    }

    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let lowered = key.to_ascii_lowercase();
                    let redacted = if ["key", "token", "secret", "password", "authorization"]
                        .iter()
                        .any(|needle| lowered.contains(needle))
                    {
                        serde_json::Value::String("[redacted]".to_string())
                    } else {
                        redact_json_for_preview(value, depth + 1)
                    };
                    (key.clone(), redacted)
                })
                .collect(),
        ),
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .iter()
                .take(8)
                .map(|value| redact_json_for_preview(value, depth + 1))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn reconcile_approval_queue(runtime_ctx: &TuiRuntimeContext, state: &mut TuiState) {
    let pending_ids = state
        .approval_queue
        .iter()
        .filter(|item| item.status == ApprovalQueueStatus::Pending)
        .map(|item| item.request_id.clone())
        .collect::<Vec<_>>();

    for request_id in pending_ids {
        if !runtime_ctx
            .approval_manager
            .has_non_cli_pending_request(&request_id)
        {
            let _ = state.update_approval_status(
                &request_id,
                ApprovalQueueStatus::Expired,
                Some("Approval expired or disappeared; runtime stayed fail-closed".to_string()),
            );
        }
    }
}

async fn refresh_workbench_views(
    runtime_ctx: &TuiRuntimeContext,
    state: &mut TuiState,
    subagent_telemetry: &SubagentTelemetryCache,
) {
    let Some(handles) = runtime_ctx.workbench.as_ref() else {
        state.set_task_board_view(None);
        state.set_subagent_pane_view(None);
        return;
    };

    let task_board = build_task_board_view(&handles.goal_engine, Some(handles.task_plan.as_ref())).await;
    state.set_task_board_view(Some(task_board));

    let subagent_view = handles
        .subagent_registry
        .as_ref()
        .map(|registry| build_subagent_pane_view(registry.as_ref(), subagent_telemetry));
    state.set_subagent_pane_view(subagent_view);
}

async fn recv_subagent_telemetry(
    runtime_ctx: &mut TuiRuntimeContext,
) -> Option<SubagentTelemetryEvent> {
    match runtime_ctx.subagent_telemetry_rx.as_mut() {
        Some(rx) => rx.recv().await,
        None => pending::<Option<SubagentTelemetryEvent>>().await,
    }
}

fn finalize_assistant_output(state: &mut TuiState, output: String) {
    if output.trim().is_empty() {
        state.finish_streaming_assistant();
        return;
    }

    if let Some(idx) = state.streaming_assistant_idx {
        if let Some(msg) = state.messages.get_mut(idx) {
            if msg.content.trim().is_empty() {
                msg.content = output;
            }
        }
        state.finish_streaming_assistant();
        return;
    }

    state.push_chat_message(TuiRole::Assistant, output);
}

#[allow(clippy::too_many_arguments)]
async fn submit_user_message(
    state: &mut TuiState,
    runtime_ctx: &mut TuiRuntimeContext,
    delta_tx: &tokio::sync::mpsc::Sender<String>,
    result_tx: &tokio::sync::mpsc::UnboundedSender<AgentTaskResult>,
    agent_event_tx: &tokio::sync::mpsc::Sender<AgentEvent>,
    approval_prompt_tx: &tokio::sync::mpsc::UnboundedSender<NonCliApprovalPrompt>,
    active_request_cancel: &mut Option<CancellationToken>,
    active_request_id: &mut Option<u64>,
    next_request_id: &mut u64,
) -> Result<()> {
    let sanitized = super::widgets::input::sanitize_input(&state.input_buffer);
    let user_text = sanitized.trim().to_string();
    if user_text.is_empty() {
        state.input_buffer.clear();
        return Ok(());
    }

    match parse_setup_command(&user_text) {
        SetupCommand::SetApiKey(api_key) => {
            state.input_buffer.clear();
            let config_path = runtime_ctx.config.config_path.display().to_string();
            match apply_inline_api_key_setup(runtime_ctx, &api_key).await {
                Ok(()) => {
                    state.provider_id = runtime_ctx.provider_name.clone();
                    state.model_id = runtime_ctx.model_name.clone();
                    state.push_chat_message(
                        TuiRole::System,
                        format!(
                            "API key saved to {} and runtime reloaded. You can continue chatting now.",
                            config_path
                        ),
                    );
                }
                Err(error) => {
                    state.push_chat_message(
                        TuiRole::Error,
                        format!("Failed to apply API key: {error}"),
                    );
                }
            }
            return Ok(());
        }
        SetupCommand::ShowHelp => {
            state.input_buffer.clear();
            state.push_chat_message(
                TuiRole::System,
                missing_api_key_guidance(&runtime_ctx.provider_name),
            );
            return Ok(());
        }
        SetupCommand::InvalidUsage(message) => {
            state.input_buffer.clear();
            state.push_chat_message(TuiRole::Error, message);
            return Ok(());
        }
        SetupCommand::None => {}
    }

    if is_provider_api_key_missing(runtime_ctx) {
        state.input_buffer.clear();
        if let Some(api_key) = parse_inline_api_key(&user_text) {
            let config_path = runtime_ctx.config.config_path.display().to_string();
            match apply_inline_api_key_setup(runtime_ctx, &api_key).await {
                Ok(()) => {
                    state.provider_id = runtime_ctx.provider_name.clone();
                    state.model_id = runtime_ctx.model_name.clone();
                    state.push_chat_message(
                        TuiRole::System,
                        format!(
                            "API key saved to {} and runtime reloaded. You can continue chatting now.",
                            config_path
                        ),
                    );
                }
                Err(error) => {
                    state.push_chat_message(
                        TuiRole::Error,
                        format!("Failed to apply API key: {error}"),
                    );
                }
            }
            return Ok(());
        }
        state.note_submitted_input(&user_text);
        state.push_chat_message(TuiRole::User, user_text);
        state.push_chat_message(
            TuiRole::Error,
            missing_api_key_guidance(&runtime_ctx.provider_name),
        );
        return Ok(());
    }

    state.note_submitted_input(&user_text);
    state.input_buffer.clear();
    state.push_chat_message(TuiRole::User, user_text.clone());
    state.awaiting_response = true;
    state.set_thinking(Some("🤔 Thinking...\n".to_string()));
    state.start_streaming_assistant();

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z");
    runtime_ctx
        .history
        .push(ChatMessage::user(format!("[{now}] {user_text}")));

    let mut task_history = std::mem::take(&mut runtime_ctx.history);
    let provider = Arc::clone(&runtime_ctx.provider);
    let observer = Arc::clone(&runtime_ctx.observer);
    let tools_registry = Arc::clone(&runtime_ctx.tools_registry);
    let provider_name = runtime_ctx.provider_name.clone();
    let model_name = runtime_ctx.model_name.clone();
    let multimodal = runtime_ctx.multimodal.clone();
    let max_tool_iterations = runtime_ctx.max_tool_iterations;
    let hooks = runtime_ctx.hooks.clone();
    let safety_heartbeat = runtime_ctx.safety_heartbeat.clone();
    let cost_enforcement = runtime_ctx.cost_enforcement.clone();
    let approval_manager = Arc::clone(&runtime_ctx.approval_manager);
    let temperature = runtime_ctx.temperature;
    let canary_tokens_enabled = runtime_ctx.canary_tokens_enabled;

    let cancel = CancellationToken::new();
    let request_id = *next_request_id;
    *next_request_id = next_request_id.saturating_add(1);
    *active_request_cancel = Some(cancel.clone());
    *active_request_id = Some(request_id);
    let child_token = cancel.child_token();
    let delta_tx = delta_tx.clone();
    let result_tx = result_tx.clone();
    let agent_event_tx = agent_event_tx.clone();
    let approval_prompt_tx = approval_prompt_tx.clone();

    // Build NonCliApprovalContext so the agent loop can request tool approvals
    // via the TUI approval modal (same pattern used by Telegram/Discord channels).
    let non_cli_approval_context = Some(NonCliApprovalContext {
        sender: TUI_APPROVAL_SENDER.to_string(),
        reply_target: TUI_APPROVAL_REPLY_TARGET.to_string(),
        prompt_tx: approval_prompt_tx,
    });

    tokio::spawn(async move {
        #[allow(clippy::large_futures)]
        let run_result = scope_agent_events(
            Some(agent_event_tx),
            scope_cost_enforcement_context(
                cost_enforcement,
                run_tool_call_loop_with_non_cli_approval_context(
                    provider.as_ref(),
                    &mut task_history,
                    tools_registry.as_slice(),
                    observer.as_ref(),
                    &provider_name,
                    &model_name,
                    temperature,
                    false,
                    Some(approval_manager.as_ref()),
                    TUI_APPROVAL_CHANNEL,
                    non_cli_approval_context,
                    &multimodal,
                    max_tool_iterations,
                    Some(child_token),
                    Some(delta_tx),
                    hooks.as_deref(),
                    &[],
                    ProgressMode::Verbose,
                    safety_heartbeat,
                    canary_tokens_enabled,
                ),
            ),
        )
        .await
        .map_err(|error| error.to_string());

        let _ = result_tx.send(AgentTaskResult {
            request_id,
            history: task_history,
            output: run_result,
        });
    });

    Ok(())
}

fn trigger_quit(
    state: &mut TuiState,
    active_request_cancel: &mut Option<CancellationToken>,
    active_request_id: &mut Option<u64>,
) {
    if let Some(cancel) = active_request_cancel.take() {
        cancel.cancel();
    }
    *active_request_id = None;
    state.should_quit = true;
}

#[derive(Debug, PartialEq, Eq)]
enum SetupCommand {
    SetApiKey(String),
    ShowHelp,
    InvalidUsage(String),
    None,
}

fn parse_setup_command(input: &str) -> SetupCommand {
    let trimmed = input.trim();
    if matches!(trimmed, "/setup" | "/help-setup" | "/setup-help") {
        return SetupCommand::ShowHelp;
    }

    let Some(rest) = trimmed.strip_prefix("/setup-key") else {
        return SetupCommand::None;
    };
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('=') {
        return SetupCommand::None;
    }
    let raw_key = rest.trim_start_matches([' ', '=']).trim();
    if raw_key.is_empty() {
        return SetupCommand::InvalidUsage("Usage: /setup-key <YOUR_API_KEY>".to_string());
    }
    let normalized_key = normalize_api_key(raw_key);
    if normalized_key.is_empty() {
        return SetupCommand::InvalidUsage("Usage: /setup-key <YOUR_API_KEY>".to_string());
    }
    SetupCommand::SetApiKey(normalized_key)
}

fn normalize_api_key(input: &str) -> String {
    input
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn parse_inline_api_key(input: &str) -> Option<String> {
    let normalized = normalize_api_key(input);
    if normalized.is_empty() {
        return None;
    }
    if !looks_like_api_key(&normalized) {
        return None;
    }
    Some(normalized)
}

fn looks_like_api_key(candidate: &str) -> bool {
    if candidate.len() < 20 {
        return false;
    }
    if candidate.chars().any(char::is_whitespace) {
        return false;
    }
    if !candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | '='))
    {
        return false;
    }
    candidate.starts_with("sk-")
        || candidate.starts_with("rk-")
        || candidate.starts_with("or-")
        || candidate.starts_with("gsk_")
        || candidate.starts_with("xai-")
        || candidate.starts_with("AIza")
        || candidate.starts_with("ya29.")
        || candidate.contains('-')
        || candidate.contains('_')
}

fn provider_requires_api_key(provider_name: &str) -> bool {
    !matches!(
        provider_name,
        "bedrock" | "aws-bedrock" | "ollama" | "llamacpp" | "llama.cpp" | "sglang" | "vllm"
    )
}

fn is_provider_api_key_missing(runtime_ctx: &TuiRuntimeContext) -> bool {
    provider_requires_api_key(&runtime_ctx.provider_name)
        && !providers::has_provider_credential(
            &runtime_ctx.provider_name,
            runtime_ctx.config.api_key.as_deref(),
        )
}

fn provider_api_key_env_var(provider_name: &str) -> &'static str {
    match provider_name {
        "openrouter" => "OPENROUTER_API_KEY",
        "openai" | "openai-codex" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "gemini" | "google" | "google-gemini" => "GEMINI_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "groq" => "GROQ_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "xai" | "grok" => "XAI_API_KEY",
        "together" | "together-ai" => "TOGETHER_API_KEY",
        "qwen" | "qwen-intl" | "dashscope-us" => "DASHSCOPE_API_KEY",
        _ => "API_KEY",
    }
}

fn missing_api_key_guidance(provider_name: &str) -> String {
    let env_var = provider_api_key_env_var(provider_name);
    format!(
        "⚠️  API key not configured for `{provider_name}`\n\n\
        Quick setup (no restart needed):\n\
        1. Paste your API key directly and press Enter\n\
        2. Or run: /setup-key <YOUR_API_KEY>\n\
        3. Env var option: {env_var}\n\n\
        You can still run `clawclawclaw onboard` later for full setup."
    )
}

async fn apply_inline_api_key_setup(
    runtime_ctx: &mut TuiRuntimeContext,
    api_key: &str,
) -> Result<()> {
    let api_key = normalize_api_key(api_key);
    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    runtime_ctx.config.api_key = Some(api_key);
    runtime_ctx.config.save().await?;

    let preserved_history = std::mem::take(&mut runtime_ctx.history);
    let mut refreshed_ctx = bootstrap_runtime(&runtime_ctx.config).await?;
    if !preserved_history.is_empty() {
        refreshed_ctx.history = preserved_history;
    }
    *runtime_ctx = refreshed_ctx;
    Ok(())
}

async fn bootstrap_runtime(config: &Config) -> Result<TuiRuntimeContext> {
    if let Err(error) = crate::plugins::runtime::initialize_from_config(&config.plugins) {
        tracing::warn!("plugin registry initialization skipped: {error}");
    }

    let observer: Arc<dyn Observer> =
        Arc::from(observability::create_observer(&config.observability));
    let runtime_adapter: Arc<dyn runtime::RuntimeAdapter> =
        Arc::from(runtime::create_runtime(&config.runtime)?);
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));
    let memory: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage(
        &config.memory,
        Some(&config.storage.provider.config),
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    let (composio_key, composio_entity_id) = if config.composio.enabled {
        (
            config.composio.api_key.as_deref(),
            Some(config.composio.entity_id.as_str()),
        )
    } else {
        (None, None)
    };

    let (subagent_telemetry_tx, subagent_telemetry_rx) =
        tokio::sync::mpsc::unbounded_channel::<SubagentTelemetryEvent>();
    let tool_bootstrap = tools::all_tools_with_runtime_and_handles(
        Arc::new(config.clone()),
        &security,
        runtime_adapter,
        memory,
        composio_key,
        composio_entity_id,
        &config.browser,
        &config.http_request,
        &config.web_fetch,
        &config.workspace_dir,
        &config.agents,
        config.api_key.as_deref(),
        config,
        Some(build_subagent_observer_factory(subagent_telemetry_tx)),
    );
    let mut tools_registry = tool_bootstrap.tools;

    let peripheral_tools: Vec<Box<dyn Tool>> =
        crate::peripherals::create_peripheral_tools(&config.peripherals).await?;
    if !peripheral_tools.is_empty() {
        tools_registry.extend(peripheral_tools);
    }

    let provider_name = config
        .default_provider
        .as_deref()
        .unwrap_or("openrouter")
        .to_string();
    let model_name =
        resolve_default_model_id(config.default_model.as_deref(), Some(&provider_name));

    let provider_runtime_options = providers::ProviderRuntimeOptions {
        auth_profile_override: None,
        provider_api_url: config.api_url.clone(),
        provider_transport: config.effective_provider_transport(),
        clawclawclaw_dir: config.config_path.parent().map(std::path::PathBuf::from),
        secrets_encrypt: config.secrets.encrypt,
        reasoning_enabled: config.runtime.reasoning_enabled,
        reasoning_level: config.effective_provider_reasoning_level(),
        custom_provider_api_mode: config.provider_api.map(|mode| mode.as_compatible_mode()),
        max_tokens_override: None,
        model_support_vision: config.model_support_vision,
    };
    let provider_box = providers::create_routed_provider_with_options(
        &provider_name,
        config.api_key.as_deref(),
        config.api_url.as_deref(),
        &config.reliability,
        &config.model_routes,
        &model_name,
        &provider_runtime_options,
    )?;
    let provider: Arc<dyn Provider> = Arc::from(provider_box);

    let tool_descs_owned: Vec<(String, String)> = tools_registry
        .iter()
        .map(|tool| (tool.name().to_string(), tool.description().to_string()))
        .collect();
    let tool_descs: Vec<(&str, &str)> = tool_descs_owned
        .iter()
        .map(|(name, desc)| (name.as_str(), desc.as_str()))
        .collect();
    let skills = crate::skills::load_skills_with_config(&config.workspace_dir, config);
    let bootstrap_max_chars = if config.agent.compact_context {
        Some(6000)
    } else {
        None
    };
    let native_tools = provider.supports_native_tools();
    let mut system_prompt = crate::channels::build_system_prompt_with_mode(
        &config.workspace_dir,
        &model_name,
        &tool_descs,
        &skills,
        Some(&config.identity),
        bootstrap_max_chars,
        native_tools,
        config.skills.prompt_injection_mode,
    );
    if !native_tools {
        system_prompt.push_str(&build_tool_instructions(&tools_registry));
    }
    system_prompt.push_str(&build_shell_policy_instructions(&config.autonomy));

    let history = vec![ChatMessage::system(system_prompt)];
    let hooks = crate::hooks::create_runner_from_config(&config.hooks);
    let safety_heartbeat = if config.agent.safety_heartbeat_interval > 0 {
        Some(SafetyHeartbeatConfig {
            body: security.summary_for_heartbeat(),
            interval: config.agent.safety_heartbeat_interval,
        })
    } else {
        None
    };
    let cost_enforcement = create_cost_enforcement_context(&config.cost, &config.workspace_dir);
    let approval_manager = Arc::new(crate::approval::ApprovalManager::from_config(
        &config.autonomy,
    ));
    let workbench = tool_bootstrap.handles.task_plan.clone().map(|task_plan| WorkbenchReadHandles {
        goal_engine: GoalEngine::new(&config.workspace_dir),
        task_plan,
        subagent_registry: tool_bootstrap.handles.subagent_registry.clone(),
    });

    Ok(TuiRuntimeContext {
        config: config.clone(),
        provider_name,
        model_name,
        provider,
        observer,
        tools_registry: Arc::new(tools_registry),
        multimodal: config.multimodal.clone(),
        max_tool_iterations: config.agent.max_tool_iterations,
        hooks,
        safety_heartbeat,
        cost_enforcement,
        approval_manager,
        temperature: config.default_temperature,
        history,
        canary_tokens_enabled: config.security.canary_tokens,
        workbench,
        subagent_telemetry_rx: Some(subagent_telemetry_rx),
    })
}

/// Convert technical error messages to user-friendly TUI messages.
fn friendly_error_message(error: &str, provider_name: &str) -> String {
    // Detect common configuration errors and provide actionable guidance
    if error.contains("API key not set") || error.contains("API key not configured") {
        return missing_api_key_guidance(provider_name);
    }

    if error.contains("All providers/models failed") {
        // Extract the first meaningful error from the attempts list
        let first_error = error
            .lines()
            .skip(1) // Skip "All providers/models failed. Attempts:"
            .find_map(|line| {
                if line.contains("error=") {
                    let error_part = line.split("error=").nth(1)?;
                    // Truncate long errors
                    let truncated = truncate_with_ellipsis(error_part, 100);
                    Some(truncated)
                } else {
                    None
                }
            });

        if let Some(specific_error) = first_error {
            // Recursively process the specific error
            return friendly_error_message(&specific_error, provider_name);
        }

        return format!(
            "⚠️  All model providers failed\n\n\
            Please check your configuration:\n\
            • Run: clawclawclaw doctor\n\
            • Verify API keys are set correctly\n\
            • Update credentials inline with /setup-key <YOUR_API_KEY>"
        )
        .to_string();
    }

    if error.contains("rate limit") || error.contains("429") {
        return format!(
            "⚠️  Rate limit exceeded\n\n\
            The AI service is temporarily limiting requests.\n\
            Please wait a moment and try again."
        )
        .to_string();
    }

    if error.contains("timeout") || error.contains("timed out") {
        return format!(
            "⚠️  Request timed out\n\n\
            The AI service took too long to respond.\n\
            Please try again."
        )
        .to_string();
    }

    if error.contains("network") || error.contains("connection") || error.contains("DNS") {
        return format!(
            "⚠️  Network error\n\n\
            Could not connect to the AI service.\n\
            Please check your internet connection."
        )
        .to_string();
    }

    // For unknown errors, show a truncated version
    let truncated = truncate_with_ellipsis(error, 200);

    format!(
        "⚠️  Error\n\n{}\n\nYou can retry after fixing the issue.",
        truncated
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::config::schema::ModelPricing;

    use super::{
        estimate_usage_cost, looks_like_api_key, parse_inline_api_key, parse_setup_command,
        truncate_json_for_display, SetupCommand,
    };

    #[test]
    fn parse_setup_key_command_extracts_key() {
        assert_eq!(
            parse_setup_command("/setup-key sk-test_value_123"),
            SetupCommand::SetApiKey("sk-test_value_123".to_string())
        );
        assert_eq!(
            parse_setup_command("/setup-key=sk-test_value_123"),
            SetupCommand::SetApiKey("sk-test_value_123".to_string())
        );
    }

    #[test]
    fn parse_setup_key_command_validates_usage() {
        assert_eq!(
            parse_setup_command("/setup-key"),
            SetupCommand::InvalidUsage("Usage: /setup-key <YOUR_API_KEY>".to_string())
        );
    }

    #[test]
    fn inline_api_key_detection_accepts_common_patterns() {
        assert!(looks_like_api_key("sk-test_value_123456789012345"));
        assert!(looks_like_api_key("AIzaSyA-test-value-1234567890"));
        assert!(parse_inline_api_key("sk-test_value_123456789012345").is_some());
        assert!(parse_inline_api_key("hello world").is_none());
    }

    #[test]
    fn truncate_json_for_display_preserves_utf8_boundaries() {
        let value = serde_json::Value::String("你好😀世界你好😀世界".to_string());

        let truncated = truncate_json_for_display(&value, 4);

        assert!(truncated.ends_with("..."));
        assert!(truncated.contains("你好"));
    }

    #[test]
    fn estimate_usage_cost_uses_model_pricing() {
        let mut prices = HashMap::new();
        prices.insert(
            "anthropic/claude-sonnet".to_string(),
            ModelPricing {
                input: 3.0,
                output: 15.0,
            },
        );

        let cost = estimate_usage_cost(
            &prices,
            "anthropic",
            "claude-sonnet",
            Some(1_000),
            Some(500),
        )
        .expect("cost should be estimated");

        assert!((cost - 0.0105).abs() < 0.0001);
    }
}
