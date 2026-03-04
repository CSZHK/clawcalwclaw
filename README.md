<p align="center">
  <img src="zero-claw.jpeg" alt="clawclawclaw" width="200" />
</p>

<h1 align="center">clawclawclaw 🦞</h1>

<p align="center">
  <strong>A concise, elegant, and powerful ultimate claw terminator, an all-in-one personalized private Agent.</strong>
</p>

<p align="center">
  <a href="LICENSE-APACHE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache%202.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
  <a href="https://github.com/CSZHK/clawcalwclaw"><img src="https://img.shields.io/badge/GitHub-CSZHK%2Fclawcalwclaw-181717?style=flat&logo=github" alt="GitHub: CSZHK/clawcalwclaw" /></a>
  <a href="https://github.com/CSZHK/clawcalwclaw/issues"><img src="https://img.shields.io/github/issues/CSZHK/clawcalwclaw?color=orange" alt="Open Issues" /></a>
  <a href="https://github.com/CSZHK/clawcalwclaw/stargazers"><img src="https://img.shields.io/github/stars/CSZHK/clawcalwclaw?style=flat" alt="GitHub Stars" /></a>
</p>

<p align="center">
  🌐 <strong>Languages:</strong>
  <a href="README.md">English</a> ·
  <a href="docs/i18n/zh-CN/README.md">简体中文</a> ·
  <a href="docs/i18n/ja/README.md">日本語</a> ·
  <a href="docs/i18n/ru/README.md">Русский</a> ·
  <a href="docs/i18n/fr/README.md">Français</a> ·
  <a href="docs/i18n/vi/README.md">Tiếng Việt</a> ·
  <a href="docs/i18n/el/README.md">Ελληνικά</a>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> |
  <a href="docs/README.md">Docs Hub</a> |
  <a href="docs/SUMMARY.md">Docs TOC</a> |
  <a href="#how-clawclawclaw-differs">How It Differs</a>
</p>

## Why clawclawclaw

`clawclawclaw` is an independently maintained fork focused on one clear target:
**a personal, private, and controllable all-in-one agent runtime**.

It keeps the Rust-first architecture and high performance foundation, while pushing a more opinionated direction for individual operators and private deployments.

## How clawclawclaw Differs

| Dimension | clawclawclaw (this project) | Upstream ZeroClaw |
|---|---|---|
| Product focus | Personalized private agent runtime for self-hosting and personal workflows | General-purpose high-performance agent framework |
| Default identity | `clawclawclaw` CLI and branding | `zeroclaw` branding |
| Default local data path | `~/.clawclawclaw` | `~/.zeroclaw` |
| Compatibility stance | Includes migration bridge from legacy `~/.zeroclaw` and legacy scope alias parsing | Upstream-native contracts |
| Roadmap ownership | Independent maintainer roadmap | Upstream maintainer/community roadmap |

## Project Characteristics

- Private-first local runtime for personal and small-team use.
- Trait-driven modular architecture (providers, channels, tools, memory, runtime adapters).
- Single-binary Rust deployment with low operational overhead.
- Strong terminal ergonomics, including TUI mode (`--features tui-ratatui`).
- Service-oriented always-on mode for long-running agent workflows.
- Broad integration surface (providers/channels/tools) while preserving explicit policy controls.

## Runtime Compatibility & Migration

### Command and naming

- Primary command: `clawclawclaw`
- Install script entry: `clawclawclaw_install.sh`
- Service names: `clawclawclaw.service` / `com.clawclawclaw.daemon`

### Config directory migration

- New default config directory: `~/.clawclawclaw`
- Legacy directory: `~/.zeroclaw`
- On default-path startup, legacy config is auto-migrated when needed.

### Environment variables

- Preferred prefix: `CLAWCLAWCLAW_...`
- Legacy compatibility is preserved where explicitly required by migration logic.

## Quick Start

### Option 1: Homebrew (macOS/Linuxbrew)

```bash
brew install clawclawclaw
```

### Option 2: Clone + Bootstrap

```bash
git clone https://github.com/CSZHK/clawcalwclaw.git
cd clawcalwclaw
./bootstrap.sh
```

### Option 3: Cargo Install

```bash
cargo install clawclawclaw
cargo install clawclawclaw --features tui-ratatui
```

### First Run

```bash
# Gateway + Web dashboard
clawclawclaw gateway

# Chat from CLI
clawclawclaw chat "Hello"

# Full-screen terminal UI
clawclawclaw tui
```

## Recommended Docs Paths

- [One-click bootstrap](docs/one-click-bootstrap.md)
- [Commands reference](docs/commands-reference.md)
- [Config reference](docs/config-reference.md)
- [Operations runbook](docs/operations-runbook.md)
- [Troubleshooting](docs/troubleshooting.md)

## Official Repository

This is the canonical repository for this project:

- <https://github.com/CSZHK/clawcalwclaw>

## Upstream Attribution

This project is based on upstream **ZeroClaw**.
Respect and thanks to the original maintainers and contributors:

- <https://github.com/zeroclaw-labs/zeroclaw>

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CLA.md](CLA.md).

## License

Dual-licensed:

- [MIT](LICENSE-MIT)
- [Apache 2.0](LICENSE-APACHE)

You may choose either license.
