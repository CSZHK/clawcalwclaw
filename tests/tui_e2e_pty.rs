//! TUI E2E tests using portable-pty for real terminal simulation.
//!
//! These tests spawn actual TUI process and verify user interactions.
//! CPU usage is limited via CARGO_BUILD_JOBS=2 and test timeout.
//!
//! Run with:
//!   CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_e2e_pty -- --test-threads=1

use std::io::{Read, Write};
use std::time::Duration;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

/// Maximum time to wait for TUI startup
const STARTUP_TIMEOUT_SECS: u64 = 30;

/// Maximum time for each interaction
const INTERACTION_TIMEOUT_SECS: u64 = 5;

/// Helper to read with timeout
fn read_with_timeout(
    reader: &mut Box<dyn Read + Send>,
    buf: &mut [u8],
    timeout_secs: u64,
) -> std::io::Result<usize> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut total = 0;

    while total < buf.len() && start.elapsed() < timeout {
        match reader.read(&mut buf[total..]) {
            Ok(0) => break, // EOF
            Ok(n) => total += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(total)
}

/// Helper to wait for child exit with timeout using try_wait
fn wait_with_timeout(
    child: &mut Box<dyn portable_pty::Child + Send + Sync>,
    timeout_secs: u64,
) -> Option<portable_pty::ExitStatus> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => return None,
        }
    }
    None
}

#[cfg(feature = "tui-ratatui")]
mod tui_e2e {
    use super::*;

    /// Test: TUI starts successfully and shows welcome message
    ///
    /// This is a smoke test to verify the TUI can initialize without crashing.
    ///
    /// **Note**: Marked as `#[ignore]` because E2E tests require compiling
    /// the full binary which is slow. Run explicitly with:
    /// ```bash
    /// CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_e2e_pty -- --test-threads=1 --ignored
    /// ```
    #[test]
    #[ignore = "E2E test: run with --ignored flag"]
    fn tui_starts_and_shows_welcome() {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to create pty");

        // Build command with limited parallelism
        let mut cmd = CommandBuilder::new("cargo");
        cmd.arg("run");
        cmd.arg("--bin");
        cmd.arg("clawclawclaw");
        cmd.arg("--features");
        cmd.arg("tui-ratatui");
        cmd.arg("--quiet");
        // Limit build parallelism via environment
        cmd.env("CARGO_BUILD_JOBS", "2");

        let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn TUI");

        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("Failed to clone reader");
        let mut writer = pair.master.take_writer().expect("Failed to take writer");

        // Read startup output with timeout
        let mut output = vec![0u8; 4096];
        let result = read_with_timeout(&mut reader, &mut output, STARTUP_TIMEOUT_SECS);
        let n = result.expect("Read timeout on TUI startup");

        // Clean up: send quit command
        let _ = writer.write_all(b"q");
        let _ = writer.flush();

        // Wait for process to exit
        let _ = child.wait();

        // Verify output contains expected content
        let output_str = String::from_utf8_lossy(&output[..n]);

        // TUI should show provider/model info or welcome message
        // The exact content depends on config, but we expect some output
        assert!(
            n > 0,
            "TUI should produce some output on startup, got empty buffer"
        );

        // Output should contain printable characters (not just escape sequences)
        let printable: String = output_str.chars().filter(|c| c.is_alphanumeric()).collect();
        assert!(
            printable.len() > 10,
            "TUI output should contain readable text, got: {} chars",
            printable.len()
        );
    }

    /// Test: TUI responds to quit command (q key)
    ///
    /// Verifies that pressing 'q' exits the TUI cleanly.
    #[test]
    #[ignore = "E2E test: run with --ignored flag"]
    fn tui_quits_on_q_key() {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to create pty");

        let mut cmd = CommandBuilder::new("cargo");
        cmd.arg("run");
        cmd.arg("--bin");
        cmd.arg("clawclawclaw");
        cmd.arg("--features");
        cmd.arg("tui-ratatui");
        cmd.arg("--quiet");
        cmd.env("CARGO_BUILD_JOBS", "2");

        let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn TUI");

        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("Failed to clone reader");
        let mut writer = pair.master.take_writer().expect("Failed to take writer");

        // Wait for startup
        let mut output = vec![0u8; 4096];
        let _ = read_with_timeout(&mut reader, &mut output, STARTUP_TIMEOUT_SECS);

        // Send quit command
        writer.write_all(b"q").expect("Failed to send q");
        writer.flush().expect("Failed to flush");

        // Wait for exit with timeout
        let exit_status = wait_with_timeout(&mut child, INTERACTION_TIMEOUT_SECS);
        assert!(
            exit_status.is_some(),
            "TUI should exit within {}s after pressing q",
            INTERACTION_TIMEOUT_SECS
        );
    }

    /// Test: TUI handles Ctrl+C gracefully
    ///
    /// Verifies that Ctrl+C cancels current operation or exits.
    #[test]
    #[ignore = "E2E test: run with --ignored flag"]
    fn tui_handles_ctrl_c() {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to create pty");

        let mut cmd = CommandBuilder::new("cargo");
        cmd.arg("run");
        cmd.arg("--bin");
        cmd.arg("clawclawclaw");
        cmd.arg("--features");
        cmd.arg("tui-ratatui");
        cmd.arg("--quiet");
        cmd.env("CARGO_BUILD_JOBS", "2");

        let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn TUI");

        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("Failed to clone reader");
        let mut writer = pair.master.take_writer().expect("Failed to take writer");

        // Wait for startup
        let mut output = vec![0u8; 4096];
        let _ = read_with_timeout(&mut reader, &mut output, STARTUP_TIMEOUT_SECS);

        // Send Ctrl+C twice (double Ctrl+C triggers quit)
        // First Ctrl+C cancels current operation
        writer.write_all(b"\x03").expect("Failed to send Ctrl+C");
        writer.flush().expect("Failed to flush");
        std::thread::sleep(Duration::from_millis(100));

        // Second Ctrl+C within 300ms triggers quit
        writer
            .write_all(b"\x03")
            .expect("Failed to send second Ctrl+C");
        writer.flush().expect("Failed to flush");

        // Wait for exit
        let exit_status = wait_with_timeout(&mut child, 3);
        assert!(exit_status.is_some(), "TUI should exit after double Ctrl+C");
    }
}

/// Fallback test when tui-ratatui feature is not enabled
#[cfg(not(feature = "tui-ratatui"))]
mod tui_e2e {
    #[test]
    fn tui_e2e_requires_feature() {
        eprintln!("TUI E2E tests require --features tui-ratatui");
        eprintln!("Run: cargo test --features tui-ratatui --test tui_e2e_pty");
    }
}
