//! Read-only workbench projections for the TUI.
//!
//! This module consumes typed authority handles and safe telemetry summaries to
//! build render-ready view models. It does not mutate runtime authority state.

use std::collections::HashMap;
use std::sync::Arc;
use chrono::Local;

use crate::goals::engine::{GoalEngine, GoalPriority, GoalState, StepStatus};
use crate::observability::traits::{Observer, ObserverEvent, ObserverMetric};
use crate::tools::{
    SubAgentRegistry, SubagentObserverFactory, TaskPlanSnapshotItem, TaskPlanSnapshotStatus,
    TaskPlanTool,
};
use crate::util::truncate_with_ellipsis;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBoardStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAuthorityKey {
    GoalStep { goal_id: String, step_id: String },
    SessionTask { task_id: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardItem {
    pub authority: TaskAuthorityKey,
    pub title: String,
    pub status: TaskBoardStatus,
    pub priority_label: Option<String>,
    pub group_label: String,
    pub detail_summary: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskBoardView {
    pub durable_items: Vec<TaskBoardItem>,
    pub session_items: Vec<TaskBoardItem>,
    pub merged_items: Vec<TaskBoardItem>,
    pub refreshed_at: String,
    pub error_summary: Option<String>,
}

impl TaskBoardView {
    pub fn has_visible_content(&self) -> bool {
        self.error_summary.is_some() || !self.merged_items.is_empty()
    }
}

pub async fn build_task_board_view(
    goal_engine: &GoalEngine,
    task_plan: Option<&TaskPlanTool>,
) -> TaskBoardView {
    let refreshed_at = Local::now().format("%H:%M:%S").to_string();
    let goal_state = match goal_engine.load_state().await {
        Ok(state) => state,
        Err(error) => {
            return TaskBoardView {
                durable_items: Vec::new(),
                session_items: Vec::new(),
                merged_items: Vec::new(),
                refreshed_at,
                error_summary: Some(format!(
                    "Goal projection unavailable: {}",
                    truncate_with_ellipsis(&error.to_string(), 120)
                )),
            };
        }
    };

    let durable_items = project_goal_items(&goal_state);
    let session_items = task_plan
        .map(TaskPlanTool::snapshot)
        .unwrap_or_default()
        .into_iter()
        .map(project_session_task)
        .collect::<Vec<_>>();

    let mut merged_items = durable_items.clone();
    merged_items.extend(session_items.clone());

    TaskBoardView {
        durable_items,
        session_items,
        merged_items,
        refreshed_at,
        error_summary: None,
    }
}

fn project_goal_items(state: &GoalState) -> Vec<TaskBoardItem> {
    let mut items = Vec::new();

    for goal in &state.goals {
        for step in &goal.steps {
            items.push(TaskBoardItem {
                authority: TaskAuthorityKey::GoalStep {
                    goal_id: goal.id.clone(),
                    step_id: step.id.clone(),
                },
                title: step.description.clone(),
                status: map_goal_step_status(step.status.clone()),
                priority_label: Some(priority_label(goal.priority)),
                group_label: goal.description.clone(),
                detail_summary: step
                    .result
                    .clone()
                    .or_else(|| goal.last_error.clone())
                    .map(|summary| truncate_with_ellipsis(&summary, 120)),
            });
        }
    }

    items
}

fn project_session_task(task: TaskPlanSnapshotItem) -> TaskBoardItem {
    TaskBoardItem {
        authority: TaskAuthorityKey::SessionTask { task_id: task.id },
        title: task.title,
        status: map_task_plan_status(task.status),
        priority_label: None,
        group_label: "Session Plan".to_string(),
        detail_summary: None,
    }
}

fn priority_label(priority: GoalPriority) -> String {
    match priority {
        GoalPriority::Low => "Low",
        GoalPriority::Medium => "Medium",
        GoalPriority::High => "High",
        GoalPriority::Critical => "Critical",
    }
    .to_string()
}

fn map_goal_step_status(status: StepStatus) -> TaskBoardStatus {
    match status {
        StepStatus::Pending => TaskBoardStatus::Pending,
        StepStatus::InProgress => TaskBoardStatus::InProgress,
        StepStatus::Completed => TaskBoardStatus::Completed,
        StepStatus::Failed => TaskBoardStatus::Failed,
        StepStatus::Blocked => TaskBoardStatus::Blocked,
    }
}

fn map_task_plan_status(status: TaskPlanSnapshotStatus) -> TaskBoardStatus {
    match status {
        TaskPlanSnapshotStatus::Pending => TaskBoardStatus::Pending,
        TaskPlanSnapshotStatus::InProgress => TaskBoardStatus::InProgress,
        TaskPlanSnapshotStatus::Completed => TaskBoardStatus::Completed,
    }
}

#[derive(Debug, Clone)]
pub struct SubagentTelemetryEvent {
    pub session_id: String,
    pub agent_name: String,
    pub event: ObserverEvent,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct SubagentTelemetryCache {
    items: HashMap<String, SubagentTelemetrySummary>,
}

#[derive(Debug, Clone, Default)]
pub struct SubagentTelemetrySummary {
    pub agent_name: String,
    pub last_event_summary: Option<String>,
    pub last_tool_name: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub error_summary: Option<String>,
    pub recorded_at: Option<String>,
}

impl SubagentTelemetryCache {
    pub fn record(&mut self, event: SubagentTelemetryEvent) {
        let entry = self.items.entry(event.session_id).or_default();
        entry.agent_name = event.agent_name;
        entry.recorded_at = Some(event.recorded_at);

        match event.event {
            ObserverEvent::AgentStart { provider, model } => {
                entry.last_event_summary = Some(format!("Started {provider}/{model}"));
            }
            ObserverEvent::LlmRequest {
                provider,
                model,
                messages_count,
            } => {
                entry.last_event_summary = Some(format!(
                    "LLM request {provider}/{model} ({messages_count} msgs)"
                ));
            }
            ObserverEvent::LlmResponse {
                success,
                error_message,
                input_tokens,
                output_tokens,
                ..
            } => {
                entry.last_event_summary = Some(if success {
                    "LLM response received".to_string()
                } else {
                    "LLM response failed".to_string()
                });
                if let Some(tokens) = input_tokens {
                    entry.input_tokens = Some(tokens);
                }
                if let Some(tokens) = output_tokens {
                    entry.output_tokens = Some(tokens);
                }
                if let Some(error) = error_message {
                    entry.error_summary = Some(truncate_with_ellipsis(&error, 120));
                }
            }
            ObserverEvent::AgentEnd { tokens_used, .. } => {
                entry.last_event_summary = Some("Agent finished".to_string());
                if let Some(tokens) = tokens_used {
                    entry.output_tokens = Some(tokens);
                }
            }
            ObserverEvent::ToolCallStart { tool } => {
                entry.last_tool_name = Some(tool.clone());
                entry.last_event_summary = Some(format!("Running {tool}"));
            }
            ObserverEvent::ToolCall {
                tool,
                success,
                duration,
            } => {
                entry.last_tool_name = Some(tool.clone());
                entry.last_event_summary = Some(format!(
                    "{} {tool} ({:.1}s)",
                    if success { "Finished" } else { "Failed" },
                    duration.as_secs_f32()
                ));
            }
            ObserverEvent::Error { component, message } => {
                entry.last_event_summary = Some(format!("Error in {component}"));
                entry.error_summary = Some(truncate_with_ellipsis(&message, 120));
            }
            ObserverEvent::TurnComplete | ObserverEvent::ChannelMessage { .. }
            | ObserverEvent::WebhookAuthFailure { .. } | ObserverEvent::HeartbeatTick => {}
        }
    }

    pub fn summary(&self, session_id: &str) -> Option<&SubagentTelemetrySummary> {
        self.items.get(session_id)
    }
}

struct TuiForwardingObserver {
    session_id: String,
    agent_name: String,
    ui_tx: tokio::sync::mpsc::UnboundedSender<SubagentTelemetryEvent>,
}

impl Observer for TuiForwardingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        let _ = self.ui_tx.send(SubagentTelemetryEvent {
            session_id: self.session_id.clone(),
            agent_name: self.agent_name.clone(),
            event: event.clone(),
            recorded_at: Local::now().format("%H:%M:%S").to_string(),
        });
    }

    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "tui-forwarding"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub fn build_subagent_observer_factory(
    ui_tx: tokio::sync::mpsc::UnboundedSender<SubagentTelemetryEvent>,
) -> SubagentObserverFactory {
    Arc::new(move |session_id: String, agent_name: String| {
        Arc::new(TuiForwardingObserver {
            session_id,
            agent_name,
            ui_tx: ui_tx.clone(),
        })
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAgentViewStatus {
    Running,
    Completed,
    Failed,
    Killed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAgentProjectionItem {
    pub session_id: String,
    pub agent_name: String,
    pub status: SubAgentViewStatus,
    pub task_summary: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub last_event_summary: Option<String>,
    pub last_tool_name: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub error_summary: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubAgentPaneView {
    pub items: Vec<SubAgentProjectionItem>,
    pub refreshed_at: String,
}

impl SubAgentPaneView {
    pub fn has_visible_content(&self) -> bool {
        !self.items.is_empty()
    }
}

pub fn build_subagent_pane_view(
    registry: &SubAgentRegistry,
    telemetry: &SubagentTelemetryCache,
) -> SubAgentPaneView {
    let refreshed_at = Local::now().format("%H:%M:%S").to_string();
    let items = registry
        .list(Some("all"))
        .into_iter()
        .map(|session| {
            let telemetry_summary = telemetry.summary(&session.session_id);
            let status_snapshot = registry.get_status(&session.session_id);
            let error_summary = status_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.result.as_ref())
                .and_then(|result| result.error.clone())
                .map(|error| truncate_with_ellipsis(&error, 120))
                .or_else(|| telemetry_summary.and_then(|summary| summary.error_summary.clone()));

            SubAgentProjectionItem {
                session_id: session.session_id.clone(),
                agent_name: session.agent,
                status: match session.status.as_str() {
                    "running" => SubAgentViewStatus::Running,
                    "completed" => SubAgentViewStatus::Completed,
                    "failed" => SubAgentViewStatus::Failed,
                    _ => SubAgentViewStatus::Killed,
                },
                task_summary: session.task,
                started_at: session.started_at,
                completed_at: session.completed_at,
                last_event_summary: telemetry_summary
                    .and_then(|summary| summary.last_event_summary.clone()),
                last_tool_name: telemetry_summary.and_then(|summary| summary.last_tool_name.clone()),
                input_tokens: telemetry_summary.and_then(|summary| summary.input_tokens),
                output_tokens: telemetry_summary.and_then(|summary| summary.output_tokens),
                error_summary,
            }
        })
        .collect();

    SubAgentPaneView { items, refreshed_at }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goals::engine::{Goal, GoalPriority, GoalState, GoalStatus, Step};
    use crate::security::SecurityPolicy;
    use crate::tools::subagent_registry::{SubAgentSession, SubAgentStatus};
    use crate::tools::traits::Tool;
    use chrono::Utc;
    use std::time::Duration;
    use tempfile::TempDir;

    fn sample_goal_state() -> GoalState {
        GoalState {
            goals: vec![Goal {
                id: "goal-1".to_string(),
                description: "Epic track".to_string(),
                status: GoalStatus::InProgress,
                priority: GoalPriority::High,
                created_at: String::new(),
                updated_at: String::new(),
                context: String::new(),
                last_error: None,
                steps: vec![
                    Step {
                        id: "step-1".to_string(),
                        description: "Design bridge".to_string(),
                        status: StepStatus::Pending,
                        result: None,
                        attempts: 0,
                    },
                    Step {
                        id: "step-2".to_string(),
                        description: "Ship UI".to_string(),
                        status: StepStatus::Blocked,
                        result: Some("waiting on runtime".to_string()),
                        attempts: 1,
                    },
                ],
            }],
        }
    }

    #[tokio::test]
    async fn task_board_view_keeps_same_title_multi_authority_items() {
        let tmp = TempDir::new().unwrap();
        let goal_engine = GoalEngine::new(tmp.path());
        goal_engine.save_state(&sample_goal_state()).await.unwrap();

        let task_plan = TaskPlanTool::new(Arc::new(SecurityPolicy::default()));
        task_plan
            .execute(serde_json::json!({
                "action": "create",
                "tasks": [{"title": "Design bridge", "status": "in_progress"}]
            }))
            .await
            .unwrap();

        let view = build_task_board_view(&goal_engine, Some(&task_plan)).await;
        assert_eq!(view.durable_items.len(), 2);
        assert_eq!(view.session_items.len(), 1);
        assert_eq!(
            view.merged_items
                .iter()
                .filter(|item| item.title == "Design bridge")
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn task_board_view_reports_goal_load_failures() {
        let tmp = TempDir::new().unwrap();
        let goal_engine = GoalEngine::new(tmp.path());
        tokio::fs::create_dir_all(tmp.path().join("state")).await.unwrap();
        tokio::fs::write(tmp.path().join("state").join("goals.json"), b"not-json")
            .await
            .unwrap();

        let view = build_task_board_view(&goal_engine, None).await;
        assert!(view.error_summary.is_some());
    }

    #[test]
    fn subagent_telemetry_cache_coalesces_updates() {
        let mut cache = SubagentTelemetryCache::default();
        cache.record(SubagentTelemetryEvent {
            session_id: "s1".to_string(),
            agent_name: "researcher".to_string(),
            event: ObserverEvent::ToolCallStart {
                tool: "shell".to_string(),
            },
            recorded_at: "10:00:00".to_string(),
        });
        cache.record(SubagentTelemetryEvent {
            session_id: "s1".to_string(),
            agent_name: "researcher".to_string(),
            event: ObserverEvent::LlmResponse {
                provider: "openai".to_string(),
                model: "gpt-5".to_string(),
                duration: Duration::from_millis(10),
                success: true,
                error_message: None,
                input_tokens: Some(12),
                output_tokens: Some(34),
            },
            recorded_at: "10:00:01".to_string(),
        });

        let summary = cache.summary("s1").unwrap();
        assert_eq!(summary.last_tool_name.as_deref(), Some("shell"));
        assert_eq!(summary.input_tokens, Some(12));
        assert_eq!(summary.output_tokens, Some(34));
    }

    #[test]
    fn subagent_pane_merges_registry_and_telemetry() {
        let registry = SubAgentRegistry::new();
        registry.insert(SubAgentSession {
            id: "s1".to_string(),
            agent_name: "researcher".to_string(),
            task: "Trace approval bridge".to_string(),
            status: SubAgentStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            result: None,
            handle: None,
        });

        let mut telemetry = SubagentTelemetryCache::default();
        telemetry.record(SubagentTelemetryEvent {
            session_id: "s1".to_string(),
            agent_name: "researcher".to_string(),
            event: ObserverEvent::ToolCall {
                tool: "file_read".to_string(),
                duration: Duration::from_secs(1),
                success: true,
            },
            recorded_at: "10:00:00".to_string(),
        });

        let view = build_subagent_pane_view(&registry, &telemetry);
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.items[0].last_tool_name.as_deref(), Some("file_read"));
        assert_eq!(view.items[0].status, SubAgentViewStatus::Running);
    }

    #[test]
    fn subagent_pane_uses_registry_terminal_error_authority() {
        let registry = SubAgentRegistry::new();
        registry.insert(SubAgentSession {
            id: "s1".to_string(),
            agent_name: "researcher".to_string(),
            task: "Trace approval bridge".to_string(),
            status: SubAgentStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            result: None,
            handle: None,
        });
        registry.fail("s1", "runtime exploded".to_string());

        let view = build_subagent_pane_view(&registry, &SubagentTelemetryCache::default());
        assert_eq!(view.items[0].status, SubAgentViewStatus::Failed);
        assert_eq!(view.items[0].error_summary.as_deref(), Some("runtime exploded"));
    }

    #[test]
    fn observer_factory_forwards_safe_events() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let factory = build_subagent_observer_factory(tx);
        let observer = factory("s1".to_string(), "coder".to_string());
        observer.record_event(&ObserverEvent::ToolCallStart {
            tool: "shell".to_string(),
        });

        let event = rx.try_recv().expect("forwarded event expected");
        assert_eq!(event.session_id, "s1");
        assert_eq!(event.agent_name, "coder");
        assert!(matches!(event.event, ObserverEvent::ToolCallStart { .. }));
    }

    #[test]
    fn task_board_view_visible_when_error_present() {
        let view = TaskBoardView {
            error_summary: Some("boom".to_string()),
            ..TaskBoardView::default()
        };
        assert!(view.has_visible_content());
    }
}
