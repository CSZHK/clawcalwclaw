//! TUI E2E tests using tmux plus an attached `script(1)` client.
//!
//! The tests keep the TUI on a real PTY, drive it through tmux key injection,
//! use `tmux capture-pane -a -p` as the primary read path, and fall back to the
//! attached client log if the pane snapshot is sparse.
//!
//! Run with:
//!   `CARGO_BUILD_JOBS=2 cargo +1.88.0 build --features tui-ratatui --bin clawclawclaw`
//!   `cargo +1.88.0 test --features tui-ratatui --test tui_e2e_pty -- --test-threads=1`

#[cfg(all(feature = "tui-ratatui", unix))]
mod tui_e2e {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Output};
    use std::sync::{Mutex, OnceLock};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use clawclawclaw::tui::widgets::sanitize::sanitize_text;
    use tempfile::TempDir;

    const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
    const INTERACTION_TIMEOUT: Duration = Duration::from_secs(5);
    const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
    const POLL_INTERVAL: Duration = Duration::from_millis(100);
    const CLEARED_ENV_VARS: &[&str] = &[
        "ANTHROPIC_API_KEY",
        "API_KEY",
        "CLAWCLAWCLAW_API_KEY",
        "CLAWCLAWCLAW_CONFIG_DIR",
        "CLAWCLAWCLAW_MODEL",
        "CLAWCLAWCLAW_PROVIDER",
        "CLAWCLAWCLAW_WORKSPACE",
        "DASHSCOPE_API_KEY",
        "DEEPSEEK_API_KEY",
        "GEMINI_API_KEY",
        "GROQ_API_KEY",
        "MISTRAL_API_KEY",
        "MODEL",
        "OPENAI_API_KEY",
        "OPENROUTER_API_KEY",
        "PROVIDER",
        "TOGETHER_API_KEY",
        "XAI_API_KEY",
    ];

    static TMUX_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[derive(Debug, Clone, Copy)]
    struct PaneStatus {
        dead: bool,
        exit_status: Option<i32>,
    }

    struct TmuxTuiHarness {
        session_name: String,
        target: String,
        temp_dir: TempDir,
        capture_log_path: PathBuf,
        attach_client: Child,
        artifact_dir: Option<PathBuf>,
        artifact_prefix: String,
    }

    impl TmuxTuiHarness {
        fn new(name: &str, cols: u16, rows: u16) -> Self {
            let temp_dir = TempDir::new().expect("failed to create temp dir");
            let home_dir = temp_dir.path().join("home");
            let config_dir = temp_dir.path().join("config");
            let workspace_dir = temp_dir.path().join("workspace");
            let capture_log_path = temp_dir.path().join("capture.log");
            fs::create_dir_all(&home_dir).expect("failed to create home dir");
            fs::create_dir_all(&config_dir).expect("failed to create config dir");
            fs::create_dir_all(&workspace_dir).expect("failed to create workspace dir");

            let binary = binary_path();
            bootstrap_config(&binary, &home_dir, &config_dir, &workspace_dir);

            let artifact_dir = std::env::var_os("TUI_ARTIFACT_DIR").map(PathBuf::from);
            let artifact_prefix = format!("{}-{}", name, unique_suffix());
            let session_name = format!("tui_{}_{}", std::process::id(), unique_suffix());
            let target = format!("{}:0.0", session_name);
            let cols_str = cols.to_string();
            let rows_str = rows.to_string();

            run_tmux([
                "new-session",
                "-d",
                "-x",
                cols_str.as_str(),
                "-y",
                rows_str.as_str(),
                "-s",
                &session_name,
            ]);
            run_tmux(["set-option", "-t", &session_name, "remain-on-exit", "on"]);

            let attach_command = format!(
                "env TERM=screen-256color tmux attach-session -t {}",
                shell_quote_str(&session_name)
            );
            let attach_client = Command::new("script")
                .env("TERM", "screen-256color")
                .args([
                    "-qefc",
                    attach_command.as_str(),
                    capture_log_path
                        .to_str()
                        .expect("capture log path must be utf-8"),
                ])
                .spawn()
                .expect("failed to spawn attached tmux client");

            thread::sleep(Duration::from_millis(200));

            let launch = build_launch_command(binary, &home_dir, &config_dir, &workspace_dir);
            run_tmux(["respawn-pane", "-k", "-t", target.as_str(), launch.as_str()]);

            Self {
                session_name,
                target,
                temp_dir,
                capture_log_path,
                attach_client,
                artifact_dir,
                artifact_prefix,
            }
        }

        fn send_literal(&self, text: &str) {
            run_tmux(["send-keys", "-t", self.target.as_str(), "-l", text]);
        }

        fn send_key(&self, key: &str) {
            run_tmux(["send-keys", "-t", self.target.as_str(), key]);
        }

        fn resize(&self, cols: u16, rows: u16) {
            let cols_str = cols.to_string();
            let rows_str = rows.to_string();
            run_tmux([
                "resize-window",
                "-t",
                &self.session_name,
                "-x",
                cols_str.as_str(),
                "-y",
                rows_str.as_str(),
            ]);
        }

        fn capture(&self) -> String {
            let pane_output = run_tmux_output([
                "capture-pane",
                "-a",
                "-p",
                "-S",
                "-",
                "-t",
                self.target.as_str(),
            ]);
            let pane_capture = normalize_capture(
                &String::from_utf8_lossy(&pane_output.stdout),
                self.temp_dir.path(),
            );
            let log_capture = normalize_capture(
                &fs::read_to_string(&self.capture_log_path).unwrap_or_default(),
                self.temp_dir.path(),
            );
            let normalized = if pane_capture.len() >= log_capture.len() {
                pane_capture
            } else {
                log_capture
            };
            if let Some(dir) = &self.artifact_dir {
                let _ = fs::create_dir_all(dir);
                let path = dir.join(format!("{}.ascii", self.artifact_prefix));
                let _ = fs::write(path, &normalized);
            }
            normalized
        }

        fn pane_status(&self) -> PaneStatus {
            let output = run_tmux_output([
                "display-message",
                "-p",
                "-t",
                self.target.as_str(),
                "#{pane_dead} #{pane_exit_status}",
            ]);
            let status = String::from_utf8_lossy(&output.stdout);
            let mut parts = status.split_whitespace();
            PaneStatus {
                dead: parts.next() == Some("1"),
                exit_status: parts.next().and_then(|value| value.parse::<i32>().ok()),
            }
        }

        fn wait_for<F>(&self, timeout: Duration, description: &str, predicate: F) -> String
        where
            F: Fn(&str) -> bool,
        {
            let start = Instant::now();
            let mut last = self.capture();
            while start.elapsed() < timeout {
                if predicate(&last) {
                    return last;
                }
                let status = self.pane_status();
                if status.dead {
                    panic!("tmux pane exited before {description}. Last capture:\n{last}");
                }
                thread::sleep(POLL_INTERVAL);
                last = self.capture();
            }
            panic!("timed out waiting for {description}. Last capture:\n{last}");
        }

        fn wait_for_contains(&self, needle: &str, timeout: Duration) -> String {
            self.wait_for(timeout, &format!("'{needle}'"), |capture| {
                contains_loose(capture, needle)
            })
        }

        fn wait_for_exit(&self, timeout: Duration) -> PaneStatus {
            let start = Instant::now();
            let mut status = self.pane_status();
            while start.elapsed() < timeout {
                if status.dead {
                    return status;
                }
                thread::sleep(POLL_INTERVAL);
                status = self.pane_status();
            }
            let last = self.capture_if_possible().unwrap_or_default();
            panic!("tmux pane did not exit in time. Last capture:\n{last}");
        }

        fn capture_if_possible(&self) -> Option<String> {
            if self.session_exists() {
                Some(self.capture())
            } else {
                None
            }
        }

        fn session_exists(&self) -> bool {
            Command::new("tmux")
                .args(["has-session", "-t", &self.session_name])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
    }

    impl Drop for TmuxTuiHarness {
        fn drop(&mut self) {
            let _ = self.attach_client.kill();
            let _ = self.attach_client.wait();
            let _ = Command::new("tmux")
                .args(["kill-session", "-t", &self.session_name])
                .output();
        }
    }

    #[test]
    #[ignore = "manual tmux harness; merge gate uses scripts/ci/run_tui_tmux_capture.sh"]
    fn startup_help_quit() {
        let _serial = test_lock();
        let Some(harness) = start_harness("startup-help-quit", 100, 28) else {
            return;
        };

        let initial = harness.wait_for_contains("clawclawclaw TUI ready", STARTUP_TIMEOUT);
        assert!(contains_loose(&initial, "Press i to edit"), "{initial}");
        assert!(
            contains_loose(&initial, "API key not configured for `openrouter`"),
            "{initial}"
        );

        harness.send_key("Escape");
        harness.send_literal("?");
        let help = harness.wait_for_contains("Keybindings", INTERACTION_TIMEOUT);
        assert!(contains_loose(&help, "Help (? to close)"), "{help}");
        assert!(contains_loose(&help, "/setup-key <KEY>"), "{help}");

        harness.send_key("Escape");
        harness.send_key("C-d");
        let status = harness.wait_for_exit(EXIT_TIMEOUT);
        assert_exit_success(status, &harness.capture_if_possible().unwrap_or_default());
    }

    #[test]
    #[ignore = "manual tmux harness; merge gate uses scripts/ci/run_tui_tmux_capture.sh"]
    fn edit_mode_setup_help_quit() {
        let _serial = test_lock();
        let Some(harness) = start_harness("setup-help-quit", 100, 28) else {
            return;
        };

        let startup = harness.wait_for_contains("clawclawclaw TUI ready", STARTUP_TIMEOUT);
        assert!(
            contains_loose(&startup, "/setup-key <YOUR_API_KEY>"),
            "{startup}"
        );

        harness.send_literal("/setup");
        harness.send_key("Enter");
        let setup = harness.wait_for(
            INTERACTION_TIMEOUT,
            "setup help to be appended",
            |capture| {
                count_occurrences(
                    &compact_for_match(capture),
                    &compact_for_match("/setup-key <YOUR_API_KEY>"),
                ) >= 2
            },
        );
        assert!(contains_loose(&setup, "Quick setup"), "{setup}");

        harness.send_key("Escape");
        harness.send_literal("?");
        let help = harness.wait_for_contains("Commands", INTERACTION_TIMEOUT);
        assert!(contains_loose(&help, "/setup"), "{help}");

        harness.send_key("Escape");
        harness.send_key("C-d");
        let status = harness.wait_for_exit(EXIT_TIMEOUT);
        assert_exit_success(status, &harness.capture_if_possible().unwrap_or_default());
    }

    #[test]
    #[ignore = "manual tmux harness; merge gate uses scripts/ci/run_tui_tmux_capture.sh"]
    fn double_ctrl_c_quit() {
        let _serial = test_lock();
        let Some(harness) = start_harness("double-ctrl-c", 100, 28) else {
            return;
        };

        harness.wait_for_contains("clawclawclaw TUI ready", STARTUP_TIMEOUT);
        harness.send_key("C-c");
        thread::sleep(Duration::from_millis(150));
        harness.send_key("C-c");
        let status = harness.wait_for_exit(EXIT_TIMEOUT);
        assert_exit_success(status, &harness.capture_if_possible().unwrap_or_default());
    }

    #[test]
    #[ignore = "manual tmux harness; merge gate uses scripts/ci/run_tui_tmux_capture.sh"]
    fn small_terminal_layout() {
        let _serial = test_lock();
        let Some(harness) = start_harness("small-layout", 100, 28) else {
            return;
        };

        harness.wait_for_contains("clawclawclaw TUI ready", STARTUP_TIMEOUT);
        harness.resize(40, 10);
        let resized = harness.wait_for(INTERACTION_TIMEOUT, "small layout render", |capture| {
            contains_loose(capture, "Chat") && contains_loose(capture, "Input")
        });
        assert!(!harness.pane_status().dead, "{resized}");

        harness.send_key("C-d");
        let status = harness.wait_for_exit(EXIT_TIMEOUT);
        assert_exit_success(status, &harness.capture_if_possible().unwrap_or_default());
    }

    fn start_harness(name: &str, cols: u16, rows: u16) -> Option<TmuxTuiHarness> {
        if !runtime_available() {
            eprintln!("Skipping tmux TUI E2E test because tmux/script are not installed.");
            return None;
        }
        Some(TmuxTuiHarness::new(name, cols, rows))
    }

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        TMUX_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn runtime_available() -> bool {
        Command::new("tmux")
            .arg("-V")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
            && Command::new("script")
                .arg("--version")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
    }

    fn binary_path() -> PathBuf {
        if let Ok(path) = std::env::var("CLAWCLAWCLAW_TUI_BIN") {
            return PathBuf::from(path);
        }
        if let Some(path) = option_env!("CARGO_BIN_EXE_clawclawclaw") {
            return PathBuf::from(path);
        }
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_clawclawclaw") {
            return PathBuf::from(path);
        }

        let exe_name = format!("clawclawclaw{}", std::env::consts::EXE_SUFFIX);
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(debug_dir) = current_exe.parent().and_then(|deps| deps.parent()) {
                let candidate = debug_dir.join(&exe_name);
                if candidate.exists() {
                    return candidate;
                }
            }
        }

        let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join(&exe_name);
        if candidate.exists() {
            return candidate;
        }

        panic!(
            "tmux TUI E2E tests require a built binary; set CLAWCLAWCLAW_TUI_BIN or build target/debug/clawclawclaw first"
        );
    }

    fn build_launch_command(
        binary: PathBuf,
        home_dir: &Path,
        config_dir: &Path,
        workspace_dir: &Path,
    ) -> String {
        let mut command = String::from("exec env");
        for key in CLEARED_ENV_VARS {
            command.push_str(" -u ");
            command.push_str(key);
        }
        command.push_str(" TERM=xterm-256color RUST_LOG=error HOME=");
        command.push_str(&shell_quote_path(home_dir));
        command.push_str(" CLAWCLAWCLAW_CONFIG_DIR=");
        command.push_str(&shell_quote_path(config_dir));
        command.push_str(" CLAWCLAWCLAW_WORKSPACE=");
        command.push_str(&shell_quote_path(workspace_dir));
        command.push(' ');
        command.push_str(&shell_quote_path(&binary));
        command.push_str(" tui --provider openrouter");
        command
    }

    fn bootstrap_config(binary: &Path, home_dir: &Path, config_dir: &Path, workspace_dir: &Path) {
        let mut command = Command::new(binary);
        command.args([
            "onboard",
            "--force",
            "--no-totp",
            "--provider",
            "openrouter",
        ]);
        for key in CLEARED_ENV_VARS {
            command.env_remove(key);
        }
        command
            .env("HOME", home_dir)
            .env("CLAWCLAWCLAW_CONFIG_DIR", config_dir)
            .env("CLAWCLAWCLAW_WORKSPACE", workspace_dir)
            .env("TERM", "xterm-256color");
        let output = command
            .output()
            .expect("failed to bootstrap config with quick setup");
        assert!(
            output.status.success(),
            "quick setup failed: stdout=\n{}\nstderr=\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_tmux<I, S>(args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = run_tmux_output(args);
        assert!(
            output.status.success(),
            "tmux command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_tmux_output<I, S>(args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("tmux")
            .args(args)
            .output()
            .expect("failed to run tmux command")
    }

    fn shell_quote_path(path: &Path) -> String {
        let raw = path.display().to_string();
        shell_quote_str(&raw)
    }

    fn shell_quote_str(raw: &str) -> String {
        format!("'{}'", raw.replace('\'', "'\"'\"'"))
    }

    fn normalize_capture(raw: &str, temp_root: &Path) -> String {
        sanitize_text(raw)
            .replace(&temp_root.display().to_string(), "<tmp>")
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    fn count_occurrences(haystack: &str, needle: &str) -> usize {
        haystack.match_indices(needle).count()
    }

    fn compact_for_match(input: &str) -> String {
        input.chars().filter(|ch| !ch.is_whitespace()).collect()
    }

    fn contains_loose(haystack: &str, needle: &str) -> bool {
        compact_for_match(haystack).contains(&compact_for_match(needle))
    }

    fn assert_exit_success(status: PaneStatus, capture: &str) {
        assert!(status.dead, "expected pane to exit\n{capture}");
        assert_eq!(
            status.exit_status,
            Some(0),
            "expected successful exit status\n{capture}"
        );
    }

    fn unique_suffix() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis()
    }
}

#[cfg(not(all(feature = "tui-ratatui", unix)))]
mod tui_e2e {
    #[test]
    fn tui_e2e_requires_feature_and_unix() {
        eprintln!("TUI E2E tests require --features tui-ratatui on a Unix host with tmux.");
        eprintln!("Run: cargo test --features tui-ratatui --test tui_e2e_pty");
    }
}
