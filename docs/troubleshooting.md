# clawclawclaw Troubleshooting

This guide focuses on common setup/runtime failures and fast resolution paths.

Last verified: **March 2, 2026**.

## Installation / Bootstrap

### `cargo` not found

Symptom:

- bootstrap exits with `cargo is not installed`

Fix:

```bash
./bootstrap.sh --install-rust
```

Or install from <https://rustup.rs/>.

### Missing system build dependencies

Symptom:

- build fails due to compiler or `pkg-config` issues

Fix:

```bash
./bootstrap.sh --install-system-deps
```

### Build fails on low-RAM / low-disk hosts

Symptoms:

- `cargo build --release` is killed (`signal: 9`, OOM killer, or `cannot allocate memory`)
- Build crashes after adding swap because disk space runs out

Why this happens:

- Runtime memory (<5MB for common operations) is not the same as compile-time memory.
- Full source build can require **2 GB RAM + swap** and **6+ GB free disk**.
- Enabling swap on a tiny disk can avoid RAM OOM but still fail due to disk exhaustion.

Preferred path for constrained machines:

```bash
./bootstrap.sh --prefer-prebuilt
```

Binary-only mode (no source fallback):

```bash
./bootstrap.sh --prebuilt-only
```

If you must compile from source on constrained hosts:

1. Add swap only if you also have enough free disk for both swap + build output.
1. Limit cargo parallelism:

```bash
CARGO_BUILD_JOBS=1 cargo build --release --locked
```

1. Reduce heavy features when Matrix is not required:

```bash
cargo build --release --locked --features hardware
```

1. Cross-compile on a stronger machine and copy the binary to the target host.

### Build is very slow or appears stuck

Symptoms:

- `cargo check` / `cargo build` appears stuck at `Checking clawclawclaw` for a long time
- repeated `Blocking waiting for file lock on package cache` or `build directory`

Why this happens in clawclawclaw:

- Matrix E2EE stack (`matrix-sdk`, `ruma`, `vodozemac`) is large and expensive to type-check.
- TLS + crypto native build scripts (`aws-lc-sys`, `ring`) add noticeable compile time.
- `rusqlite` with bundled SQLite compiles C code locally.
- Running multiple cargo jobs/worktrees in parallel causes lock contention.

Fast checks:

```bash
cargo check --timings
cargo tree -d
```

The timing report is written to `target/cargo-timings/cargo-timing.html`.

Faster local iteration (when Matrix channel is not needed):

```bash
cargo check
```

This uses the lean default feature set and can significantly reduce compile time.

To build with Matrix support explicitly enabled:

```bash
cargo check --features channel-matrix
```

To build with Matrix + Lark + hardware support:

```bash
cargo check --features hardware,channel-matrix,channel-lark
```

Lock-contention mitigation:

```bash
pgrep -af "cargo (check|build|test)|cargo check|cargo build|cargo test"
```

Stop unrelated cargo jobs before running your own build.

### `clawclawclaw` command not found after install

Symptom:

- install succeeds but shell cannot find `clawclawclaw`

Fix:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
which clawclawclaw
```

Persist in your shell profile if needed.

## Runtime / Gateway

### Windows: shell tool unavailable or repeated shell failures

Symptoms:

- agent repeatedly fails shell calls and stops early
- shell-based actions fail even though clawclawclaw starts
- `clawclawclaw doctor` reports runtime shell capability unavailable

Why this happens:

- Native Windows shell availability differs by machine setup.
- Some environments do not have `sh` in `PATH`.
- If both Git Bash and PowerShell are missing/misconfigured, shell tool execution will fail.

What changed in clawclawclaw:

- Native runtime now resolves shell with Windows fallbacks in this order:
  - `bash` -> `sh` -> `pwsh` -> `powershell` -> `cmd`/`COMSPEC`
- `clawclawclaw doctor` now reports:
  - selected native shell (kind + resolved executable path)
  - candidate shell availability on Windows
  - explicit warning when fallback is only `cmd`
- WSL2 is optional, not required.

Checks (PowerShell):

```powershell
where.exe bash
where.exe pwsh
where.exe powershell
echo $env:COMSPEC
clawclawclaw doctor
```

Fix:

1. Install at least one preferred shell:
   - Git Bash (recommended for Unix-like command compatibility), or
   - PowerShell 7 (`pwsh`)
2. Confirm the shell executable is available in `PATH`.
3. Ensure `COMSPEC` is set (normally points to `cmd.exe` on Windows).
4. Reopen terminal and rerun `clawclawclaw doctor`.

Notes:

- Running with only `cmd` fallback can work, but compatibility is lower than Git Bash or PowerShell.
- If you already use WSL2, it can help with Unix-style workflows, but it is not mandatory for clawclawclaw shell tooling.

### Gateway unreachable

Checks:

```bash
clawclawclaw status
clawclawclaw doctor
```

Verify `~/.clawclawclaw/config.toml`:

- `[gateway].host` (default `127.0.0.1`)
- `[gateway].port` (default `42617`)
- `allow_public_bind` only when intentionally exposing LAN/public interfaces

### Pairing / auth failures on webhook

Checks:

1. Ensure pairing completed (`/pair` flow)
2. Ensure bearer token is current
3. Re-run diagnostics:

```bash
clawclawclaw doctor
```

## Channel Issues

### Telegram conflict: `terminated by other getUpdates request`

Cause:

- multiple pollers using same bot token

Fix:

- keep only one active runtime for that token
- stop extra `clawclawclaw daemon` / `clawclawclaw channel start` processes

### Channel unhealthy in `channel doctor`

Checks:

```bash
clawclawclaw channel doctor
```

Then verify channel-specific credentials + allowlist fields in config.

## Web Access Issues

### `curl`/`wget` blocked in shell tool

Symptom:

- tool output includes `Command blocked: high-risk command is disallowed by policy`
- model says `curl`/`wget` is blocked

Why this happens:

- `curl`/`wget` are high-risk shell commands and may be blocked by autonomy policy.

Preferred fix:

- use purpose-built tools instead of shell fetch:
  - `http_request` for direct API/HTTP calls
  - `web_fetch` for page content extraction/summarization

Minimal config:

```toml
[http_request]
enabled = true
allowed_domains = ["*"]

[web_fetch]
enabled = true
provider = "fast_html2md"
allowed_domains = ["*"]
```

### `web_search_tool` fails with `403`/`429`

Symptom:

- tool output includes `DuckDuckGo search failed with status: 403` (or `429`)

Why this happens:

- some networks/proxies/rate limits block DuckDuckGo HTML search endpoint traffic.

Fix options:

1. Switch provider to Brave (recommended when you have an API key):

```toml
[web_search]
enabled = true
provider = "brave"
brave_api_key = "<SECRET>"
```

2. Switch provider to Firecrawl (if enabled in your build):

```toml
[web_search]
enabled = true
provider = "firecrawl"
api_key = "<SECRET>"
```

3. Keep DuckDuckGo for search, but use `web_fetch` to read pages once you have URLs.

### `web_fetch`/`http_request` says host is not allowed

Symptom:

- errors like `Host '<domain>' is not in http_request.allowed_domains`
- or `web_fetch tool is enabled but no allowed_domains are configured`

Fix:

- include exact domains or `"*"` for public internet access:

```toml
[http_request]
enabled = true
allowed_domains = ["*"]

[web_fetch]
enabled = true
allowed_domains = ["*"]
blocked_domains = []
```

Security notes:

- local/private network targets are blocked even with `"*"`
- keep explicit domain allowlists in production environments when possible

## Service Mode

### Service installed but not running

Checks:

```bash
clawclawclaw service status
```

Recovery:

```bash
clawclawclaw service stop
clawclawclaw service start
```

Linux logs:

```bash
journalctl --user -u clawclawclaw.service -f
```

## macOS Catalina (10.15) Compatibility

### Build or run fails on macOS Catalina

Symptoms:

- `cargo build` fails with linker errors referencing a minimum deployment target higher than 10.15
- Binary exits immediately or crashes with `Illegal instruction: 4` on launch
- Error message references `macOS 11.0` or `Big Sur` as a requirement

Why this happens:

- `wasmtime` (the WASM plugin engine used by the `wasm-tools` feature) uses Cranelift JIT
  compilation, which has macOS version dependencies that may exceed Catalina (10.15).
- If your Rust toolchain was installed or updated on a newer macOS host, the default
  `MACOSX_DEPLOYMENT_TARGET` may be set higher than 10.15, producing binaries that refuse
  to start on Catalina.

Fix — build without the WASM plugin engine (recommended on Catalina):

```bash
cargo build --release --locked
```

The default feature set no longer includes `wasm-tools`, so the above command produces a
Catalina-compatible binary without Cranelift/JIT dependencies.

If you need WASM plugin support and are on a newer macOS (11.0+), opt in explicitly:

```bash
cargo build --release --locked --features wasm-tools
```

Fix — explicit deployment target (belt-and-suspenders):

If you still see deployment-target linker errors, set the target explicitly before building:

```bash
MACOSX_DEPLOYMENT_TARGET=10.15 cargo build --release --locked
```

The `.cargo/config.toml` in this repository already pins `x86_64-apple-darwin` builds to
`-mmacosx-version-min=10.15`, so the environment variable is usually not required.

## Legacy Installer Compatibility

Both still work:

```bash
curl -fsSL https://raw.githubusercontent.com/CSZHK/clawcalwclaw/main/scripts/bootstrap.sh | bash
curl -fsSL https://raw.githubusercontent.com/CSZHK/clawcalwclaw/main/scripts/install.sh | bash
```

`install.sh` is a compatibility entry and forwards/falls back to bootstrap behavior.

## TUI Issues

### TUI won't start / terminal not supported

Symptoms:

- `cargo run --bin clawclawclaw --features tui-ratatui` fails with terminal errors
- TUI renders incorrectly or not at all

Fix:

1. Ensure terminal supports true color and alternate screen:

```bash
# Check terminal capabilities
echo $TERM
infocmp $TERM | grep -E "colors|cup|smcup"
```

2. Try setting TERM explicitly:

```bash
TERM=xterm-256color cargo run --bin clawclawclaw --features tui-ratatui
```

3. For SSH sessions, ensure PTY allocation:

```bash
ssh -t user@host  # Note the -t flag
```

### TUI tests fail to compile

Symptoms:

- `cargo test --features tui-ratatui` fails with feature errors

Fix:

Ensure `tui-ratatui` feature is specified:

```bash
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_render_test
```

### TUI tests timeout / hang

Symptoms:

- E2E tests hang indefinitely
- Tests exceed 30-second startup timeout

Fix:

1. First run compiles the binary (slow). Run again after compilation:

```bash
# First run (compiles binary, may timeout)
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_e2e_pty -- --test-threads=1 --ignored

# Second run (uses cached binary)
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_e2e_pty -- --test-threads=1 --ignored
```

2. Limit CPU to reduce resource contention:

```bash
CARGO_BUILD_JOBS=1 cargo test --features tui-ratatui -- --test-threads=1
```

3. Run L1/L2 tests instead (faster):

```bash
cargo test --lib tui  # L1 unit tests (seconds)
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_render_test --test tui_event_handling_test  # L2 (minutes)
```

### TUI keyboard input not working

Symptoms:

- Key presses don't register
- Wrong characters appear

Fix:

1. Ensure terminal raw mode is supported
2. Check for terminal multiplexer conflicts (tmux/screen)
3. Try without multiplexer:

```bash
# Exit tmux/screen and run directly
cargo run --bin clawclawclaw --features tui-ratatui
```

For detailed TUI testing guide, see [tui-testing.md](tui-testing.md).

## Still Stuck?

Collect and include these outputs when filing an issue:

```bash
clawclawclaw --version
clawclawclaw status
clawclawclaw doctor
clawclawclaw channel doctor
```

Also include OS, install method, and sanitized config snippets (no secrets).

## Related Docs

- [operations-runbook.md](operations-runbook.md)
- [one-click-bootstrap.md](one-click-bootstrap.md)
- [channels-reference.md](channels-reference.md)
- [network-deployment.md](network-deployment.md)
