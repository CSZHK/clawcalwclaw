# clawclawclaw Commands Reference

This reference is derived from the current CLI surface (`clawclawclaw --help`).

Last verified: **March 3, 2026**.

## Top-Level Commands

| Command | Purpose |
|---|---|
| `onboard` | Initialize workspace/config quickly or interactively |
| `agent` | Run interactive chat or single-message mode |
| `tui` | Run full-screen terminal UI mode (requires `tui-ratatui` feature) |
| `gateway` | Start webhook and WhatsApp HTTP gateway |
| `daemon` | Start supervised runtime (gateway + channels + optional heartbeat/scheduler) |
| `service` | Manage user-level OS service lifecycle |
| `doctor` | Run diagnostics and freshness checks |
| `status` | Print current configuration and system summary |
| `update` | Check or install latest clawclawclaw release |
| `estop` | Engage/resume emergency stop levels and inspect estop state |
| `cron` | Manage scheduled tasks |
| `models` | Refresh provider model catalogs |
| `providers` | List provider IDs, aliases, and active provider |
| `providers-quota` | Check provider quota usage, rate limits, and health |
| `channel` | Manage channels and channel health checks |
| `integrations` | Inspect integration details |
| `skills` | List/install/remove skills |
| `migrate` | Import from external runtimes (currently OpenClaw) |
| `config` | Inspect, query, and modify runtime configuration |
| `completions` | Generate shell completion scripts to stdout |
| `hardware` | Discover and introspect USB hardware |
| `peripheral` | Configure and flash peripherals |

## Command Groups

### `onboard`

- `clawclawclaw onboard`
- `clawclawclaw onboard --interactive`
- `clawclawclaw onboard --channels-only`
- `clawclawclaw onboard --force`
- `clawclawclaw onboard --api-key <KEY> --provider <ID> --memory <sqlite|lucid|markdown|none>`
- `clawclawclaw onboard --api-key <KEY> --provider <ID> --model <MODEL_ID> --memory <sqlite|lucid|markdown|none>`
- `clawclawclaw onboard --api-key <KEY> --provider <ID> --model <MODEL_ID> --memory <sqlite|lucid|markdown|none> --force`
- `clawclawclaw onboard --migrate-openclaw`
- `clawclawclaw onboard --migrate-openclaw --openclaw-source <PATH> --openclaw-config <PATH>`

`onboard` safety behavior:

- If `config.toml` already exists and you run `--interactive`, onboarding now offers two modes:
  - Full onboarding (overwrite `config.toml`)
  - Provider-only update (update provider/model/API key while preserving existing channels, tunnel, memory, hooks, and other settings)
- In non-interactive environments, existing `config.toml` causes a safe refusal unless `--force` is passed.
- Use `clawclawclaw onboard --channels-only` when you only need to rotate channel tokens/allowlists.
- OpenClaw migration mode is merge-first by design: existing clawclawclaw data/config is preserved, missing fields are filled, and list-like values are union-merged with de-duplication.
- Interactive onboarding can auto-detect `~/.openclaw` and prompt for optional merge migration even without `--migrate-openclaw`.

### `agent`

- `clawclawclaw agent`
- `clawclawclaw agent -m "Hello"`
- `clawclawclaw agent --provider <ID> --model <MODEL> --temperature <0.0-2.0>`
- `clawclawclaw agent --peripheral <board:path>`

Tip:

- In interactive chat, you can ask for route changes in natural language (for example “conversation uses kimi, coding uses gpt-5.3-codex”); the assistant can persist this via tool `model_routing_config`.
- In interactive chat, you can also ask for runtime orchestration changes in natural language (for example “disable agent teams”, “enable subagents”, “set max concurrent subagents to 24”, “use least_loaded strategy”); the assistant can persist this via `model_routing_config` action `set_orchestration`.
- In interactive chat, you can also ask to:
  - switch web search provider/fallbacks (`web_search_config`)
  - inspect or update domain access policy (`web_access_config`)
  - preview/apply OpenClaw merge migration (`openclaw_migration`)

### `tui`

- `clawclawclaw tui`
- `clawclawclaw tui --provider <ID> --model <MODEL>`

Notes:

- Build-time feature gate: `tui` requires compiling with `--features tui-ratatui`.
- Without this feature, `clawclawclaw tui` returns a friendly rebuild hint.
- Keyboard behavior:
  - `Enter` sends message (editing mode)
  - `Shift+Enter` inserts newline
  - `Ctrl+C` cancels current in-flight request
  - double `Ctrl+C` within 300ms forces quit
  - `q` or `Ctrl+D` exits TUI

### `gateway` / `daemon`

- `clawclawclaw gateway [--host <HOST>] [--port <PORT>] [--new-pairing]`
- `clawclawclaw daemon [--host <HOST>] [--port <PORT>]`

`--new-pairing` clears all stored paired tokens and forces generation of a fresh pairing code on gateway startup.

### `estop`

- `clawclawclaw estop` (engage `kill-all`)
- `clawclawclaw estop --level network-kill`
- `clawclawclaw estop --level domain-block --domain "*.chase.com" [--domain "*.paypal.com"]`
- `clawclawclaw estop --level tool-freeze --tool shell [--tool browser]`
- `clawclawclaw estop status`
- `clawclawclaw estop resume`
- `clawclawclaw estop resume --network`
- `clawclawclaw estop resume --domain "*.chase.com"`
- `clawclawclaw estop resume --tool shell`
- `clawclawclaw estop resume --otp <123456>`

Notes:

- `estop` commands require `[security.estop].enabled = true`.
- When `[security.estop].require_otp_to_resume = true`, `resume` requires OTP validation.
- OTP prompt appears automatically if `--otp` is omitted.

### `service`

- `clawclawclaw service install`
- `clawclawclaw service start`
- `clawclawclaw service stop`
- `clawclawclaw service restart`
- `clawclawclaw service status`
- `clawclawclaw service uninstall`

### `update`

- `clawclawclaw update --check` (check for new release, no install)
- `clawclawclaw update` (install latest release binary for current platform)
- `clawclawclaw update --force` (reinstall even if current version matches latest)
- `clawclawclaw update --instructions` (print install-method-specific guidance)

Notes:

- If clawclawclaw is installed via Homebrew, prefer `brew upgrade clawclawclaw`.
- `update --instructions` detects common install methods and prints the safest path.

### `cron`

- `clawclawclaw cron list`
- `clawclawclaw cron add <expr> [--tz <IANA_TZ>] <command>`
- `clawclawclaw cron add-at <rfc3339_timestamp> <command>`
- `clawclawclaw cron add-every <every_ms> <command>`
- `clawclawclaw cron once <delay> <command>`
- `clawclawclaw cron remove <id>`
- `clawclawclaw cron pause <id>`
- `clawclawclaw cron resume <id>`

Notes:

- Mutating schedule/cron actions require `cron.enabled = true`.
- Shell command payloads for schedule creation (`create` / `add` / `once`) are validated by security command policy before job persistence.

### `models`

- `clawclawclaw models refresh`
- `clawclawclaw models refresh --provider <ID>`
- `clawclawclaw models refresh --force`

`models refresh` currently supports live catalog refresh for provider IDs: `openrouter`, `openai`, `anthropic`, `groq`, `mistral`, `deepseek`, `xai`, `together-ai`, `gemini`, `ollama`, `llamacpp`, `sglang`, `vllm`, `astrai`, `venice`, `fireworks`, `cohere`, `moonshot`, `stepfun`, `glm`, `zai`, `qwen`, `volcengine` (`doubao`/`ark` aliases), `siliconflow`, and `nvidia`.

#### Live model availability test

```bash
./dev/test_models.sh              # test all Gemini models + profile rotation
./dev/test_models.sh models       # test model availability only
./dev/test_models.sh profiles     # test profile rotation only
```

Runs a Rust integration test (`tests/gemini_model_availability.rs`) that verifies each model against the OAuth endpoint (cloudcode-pa). Requires valid Gemini OAuth credentials in `auth-profiles.json`.

### `providers-quota`

- `clawclawclaw providers-quota` — show quota status for all configured providers
- `clawclawclaw providers-quota --provider gemini` — show quota for a specific provider
- `clawclawclaw providers-quota --format json` — JSON output for scripting

Displays provider quota usage, rate limits, circuit breaker state, and OAuth profile health.

### `doctor`

- `clawclawclaw doctor`
- `clawclawclaw doctor models [--provider <ID>] [--use-cache]`
- `clawclawclaw doctor traces [--limit <N>] [--event <TYPE>] [--contains <TEXT>]`
- `clawclawclaw doctor traces --id <TRACE_ID>`

Provider connectivity matrix CI/local helper:

- `python3 scripts/ci/provider_connectivity_matrix.py --binary target/release-fast/clawclawclaw --contract .github/connectivity/probe-contract.json`

`doctor traces` reads runtime tool/model diagnostics from `observability.runtime_trace_path`.

### `channel`

- `clawclawclaw channel list`
- `clawclawclaw channel start`
- `clawclawclaw channel doctor`
- `clawclawclaw channel bind-telegram <IDENTITY>`
- `clawclawclaw channel add <type> <json>`
- `clawclawclaw channel remove <name>`

Runtime in-chat commands while channel server is running:

- Telegram/Discord sender-session routing:
  - `/models`
  - `/models <provider>`
  - `/model`
  - `/model <model-id>`
  - `/new`
- Supervised tool approvals (all non-CLI channels):
  - `/approve-request <tool-name>` (create pending approval request)
  - `/approve-confirm <request-id>` (confirm pending request; same sender + same chat/channel only)
  - `/approve-allow <request-id>` (approve current pending runtime execution request once; no policy persistence)
  - `/approve-deny <request-id>` (deny current pending runtime execution request)
  - `/approve-pending` (list pending requests in current sender+chat/channel scope)
  - `/approve <tool-name>` (direct one-step grant + persist to `autonomy.auto_approve`, compatibility path)
  - `/unapprove <tool-name>` (revoke + remove from `autonomy.auto_approve`)
  - `/approvals` (show runtime + persisted approval state)
  - Natural-language approval behavior is controlled by `[autonomy].non_cli_natural_language_approval_mode`:
    - `direct` (default): `授权工具 shell` / `approve tool shell` immediately grants
    - `request_confirm`: natural-language approval creates pending request, then confirm with request ID
    - `disabled`: natural-language approval commands are ignored (slash commands only)
  - Optional per-channel override: `[autonomy].non_cli_natural_language_approval_mode_by_channel`

Approval safety behavior:

- Runtime approval commands are parsed and executed **before** LLM inference in the channel loop.
- Pending requests are sender+chat/channel scoped and expire automatically.
- Confirmation requires the same sender in the same chat/channel that created the request.
- Once approved and persisted, the tool remains approved across restarts until revoked.
- Optional policy gate: `[autonomy].non_cli_approval_approvers` can restrict who may execute approval-management commands.

Startup behavior for multiple channels:
- `clawclawclaw channel start` starts all configured channels in one process.
- If one channel fails initialization, other channels continue to start.
- If all configured channels fail initialization, startup exits with an error.

Channel runtime also watches `config.toml` and hot-applies updates to:
- `default_provider`
- `default_model`
- `default_temperature`
- `api_key` / `api_url` (for the default provider)
- `reliability.*` provider retry settings

`add/remove` currently route you back to managed setup/manual config paths (not full declarative mutators yet).

### `integrations`

- `clawclawclaw integrations info <name>`

### `skills`

- `clawclawclaw skills list`
- `clawclawclaw skills audit <source_or_name>`
- `clawclawclaw skills install <source>`
- `clawclawclaw skills remove <name>`

`<source>` accepts:

| Format | Example | Notes |
|---|---|---|
| **ClawhHub profile URL** | `https://clawhub.ai/steipete/summarize` | Auto-detected by domain; downloads zip from ClawhHub API |
| **ClawhHub short prefix** | `clawhub:summarize` | Short form; slug is the skill name on ClawhHub |
| **Direct zip URL** | `zip:https://example.com/skill.zip` | Any HTTPS URL returning a zip archive |
| **Local zip file** | `/path/to/skill.zip` | Zip file already downloaded to local disk |
| **Registry packages** | `namespace/name` or `namespace/name@version` | Fetched from the configured registry (default: ZeroMarket) |
| **Git remotes** | `https://github.com/…`, `git@host:owner/repo.git` | Cloned with `git clone --depth 1` |
| **Local filesystem paths** | `./my-skill` or `/abs/path/skill` | Directory copied and audited |

**ClawhHub install examples:**

```bash
# Install by profile URL (slug extracted from last path segment)
clawclawclaw skill install https://clawhub.ai/steipete/summarize

# Install using short prefix
clawclawclaw skill install clawhub:summarize

# Install from a zip already downloaded locally
clawclawclaw skill install ~/Downloads/summarize-1.0.0.zip
```

If the ClawhHub API returns 429 (rate limit) or requires authentication, set `clawhub_token` in `[skills]` config (see [config reference](config-reference.md#skills)).

**Zip-based install behavior:**
- If the zip contains `_meta.json` (OpenClaw convention), name/version/author are read from it.
- A minimal `SKILL.toml` is written automatically if neither `SKILL.toml` nor `SKILL.md` is present in the zip.

Registry packages are installed to `~/.clawclawclaw/workspace/skills/<name>/`.

`skills install` always runs a built-in static security audit before the skill is accepted. The audit blocks:
- symlinks inside the skill package
- script-like files (`.sh`, `.bash`, `.zsh`, `.ps1`, `.bat`, `.cmd`)
- high-risk command snippets (for example pipe-to-shell payloads)
- markdown links that escape the skill root, point to remote markdown, or target script files

> **Note:** The security audit applies to directory-based installs (local paths, git remotes). Zip-based installs (ClawhHub, direct zip URLs, local zip files) perform path-traversal safety checks during extraction but do not run the full static audit — review zip contents manually for untrusted sources.

Use `skills audit` to manually validate a candidate skill directory (or an installed skill by name) before sharing it.

Workspace symlink policy:
- Symlinked entries under `~/.clawclawclaw/workspace/skills/` are blocked by default.
- To allow shared local skill directories, set `[skills].trusted_skill_roots` in `config.toml`.
- A symlinked skill is accepted only when its resolved canonical target is inside one of the trusted roots.

Skill manifests (`SKILL.toml`) support `prompts` and `[[tools]]`; both are injected into the agent system prompt at runtime, so the model can follow skill instructions without manually reading skill files.

### `migrate`

- `clawclawclaw migrate openclaw [--source <path>] [--source-config <path>] [--dry-run] [--no-memory] [--no-config]`

`migrate openclaw` behavior:

- Default mode migrates both memory and config/agents with merge-first semantics.
- Existing clawclawclaw values are preserved; migration does not overwrite existing user content.
- Memory migration de-duplicates repeated content during merge while keeping existing entries intact.
- `--dry-run` prints a migration report without writing data.
- `--no-memory` or `--no-config` scopes migration to selected modules.

### `config`

- `clawclawclaw config show`
- `clawclawclaw config get <key>`
- `clawclawclaw config set <key> <value>`
- `clawclawclaw config schema`

`config show` prints the full effective configuration as pretty JSON with secrets masked as `***REDACTED***`. Environment variable overrides are already applied.

`config get <key>` queries a single value by dot-separated path (e.g. `gateway.port`, `security.estop.enabled`). Scalars print raw values; objects and arrays print pretty JSON. Sensitive fields are masked.

`config set <key> <value>` updates a configuration value and persists it atomically to `config.toml`. Types are inferred automatically (`true`/`false` → bool, integers, floats, JSON syntax → object/array, otherwise string). Type mismatches are rejected before writing.

`config schema` prints a JSON Schema (draft 2020-12) for the full `config.toml` contract to stdout.

### `completions`

- `clawclawclaw completions bash`
- `clawclawclaw completions fish`
- `clawclawclaw completions zsh`
- `clawclawclaw completions powershell`
- `clawclawclaw completions elvish`

`completions` is stdout-only by design so scripts can be sourced directly without log/warning contamination.

### `hardware`

- `clawclawclaw hardware discover`
- `clawclawclaw hardware introspect <path>`
- `clawclawclaw hardware info [--chip <chip_name>]`

### `peripheral`

- `clawclawclaw peripheral list`
- `clawclawclaw peripheral add <board> <path>`
- `clawclawclaw peripheral flash [--port <serial_port>]`
- `clawclawclaw peripheral setup-uno-q [--host <ip_or_host>]`
- `clawclawclaw peripheral flash-nucleo`

## Validation Tip

To verify docs against your current binary quickly:

```bash
clawclawclaw --help
clawclawclaw <command> --help
```
