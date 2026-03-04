<p align="center">
  <img src="zero-claw.jpeg" alt="clawclawclaw" width="200" />
</p>

<h1 align="center">clawclawclaw 🦞</h1>

<p align="center">
  <strong>A concise, elegant, and powerful ultimate claw terminator, an all-in-one personalized private Agent 🦞</strong><br>
  ⚡️ <strong>Runs on any hardware with <5MB RAM: That's 99% less memory than OpenClaw and 98% cheaper than a Mac mini!</strong>
</p>

<p align="center">
  <a href="LICENSE-APACHE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache%202.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
  <a href="NOTICE"><img src="https://img.shields.io/github/contributors/CSZHK/clawcalwclaw?color=green" alt="Contributors" /></a>
  <a href="https://github.com/CSZHK/clawcalwclaw"><img src="https://img.shields.io/badge/GitHub-CSZHK%2Fclawcalwclaw-181717?style=flat&logo=github" alt="GitHub: CSZHK/clawcalwclaw" /></a>
  <a href="https://github.com/CSZHK/clawcalwclaw/issues"><img src="https://img.shields.io/github/issues/CSZHK/clawcalwclaw?color=orange" alt="Open Issues" /></a>
  <a href="https://github.com/CSZHK/clawcalwclaw/stargazers"><img src="https://img.shields.io/github/stars/CSZHK/clawcalwclaw?style=flat" alt="GitHub Stars" /></a>
</p>
<p align="center">
Built by students and members of the Harvard, MIT, and Sundai.Club communities.
</p>

<p align="center">
  🌐 <strong>Languages:</strong> <a href="README.md">English</a> · <a href="docs/i18n/zh-CN/README.md">简体中文</a> · <a href="docs/i18n/ja/README.md">日本語</a> · <a href="docs/i18n/ru/README.md">Русский</a> · <a href="docs/i18n/fr/README.md">Français</a> · <a href="docs/i18n/vi/README.md">Tiếng Việt</a> · <a href="docs/i18n/el/README.md">Ελληνικά</a>
</p>

<p align="center">
  <a href="#quick-start">Getting Started</a> |
  <a href="docs/one-click-bootstrap.md">One-Click Setup</a> |
  <a href="docs/README.md">Docs Hub</a> |
  <a href="docs/SUMMARY.md">Docs TOC</a>
</p>

<p align="center">
  <strong>Quick Routes:</strong>
  <a href="docs/reference/README.md">Reference</a> ·
  <a href="docs/operations/README.md">Operations</a> ·
  <a href="docs/troubleshooting.md">Troubleshoot</a> ·
  <a href="docs/security/README.md">Security</a> ·
  <a href="docs/hardware/README.md">Hardware</a> ·
  <a href="docs/contributing/README.md">Contribute</a>
</p>

<p align="center">
  <strong>Fast, small, and fully autonomous Framework</strong><br />
  Deploy anywhere. Swap anything.
</p>

<p align="center">
  clawclawclaw is the <strong>runtime framework</strong> for agentic workflows — infrastructure that abstracts models, tools, memory, and execution so agents can be built once and run anywhere.
</p>

<p align="center"><code>Trait-driven architecture · secure-by-default runtime · provider/channel/tool swappable · pluggable everything</code></p>

### 📢 Announcements

Use this board for important notices (breaking changes, security advisories, maintenance windows, and release blockers).

| Date (UTC) | Level       | Notice                                                                                                                                                                                                                                                                                                                                                 | Action                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-03-04 | _Important_ | clawclawclaw is independently maintained as a personal fork.                                                                                                                                                                                                                                                                                           | Use [CSZHK/clawcalwclaw](https://github.com/CSZHK/clawcalwclaw) as the canonical source for code, issues, and releases.                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 2026-02-19 | _Important_ | Anthropic updated the Authentication and Credential Use terms on 2026-02-19. Claude Code OAuth tokens (Free, Pro, Max) are intended exclusively for Claude Code and Claude.ai; using OAuth tokens from Claude Free/Pro/Max in any other product, tool, or service (including Agent SDK) is not permitted and may violate the Consumer Terms of Service. | Please temporarily avoid Claude Code OAuth integrations to prevent potential loss. Original clause: [Authentication and Credential Use](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use).                                                                                                                                                                                                                                                                                                                                                                                    |

### Upstream Acknowledgement

This project is independently maintained as **clawclawclaw**, and is based on upstream **ZeroClaw** work.  
Respect and thanks to the original maintainers: <https://github.com/zeroclaw-labs/zeroclaw>.

### ✨ Features

- 🏎️ **Lean Runtime by Default:** Common CLI and status workflows run in a few-megabyte memory envelope on release builds.
- 💰 **Cost-Efficient Deployment:** Designed for low-cost boards and small cloud instances without heavyweight runtime dependencies.
- ⚡ **Fast Cold Starts:** Single-binary Rust runtime keeps command and daemon startup near-instant for daily operations.
- 🌍 **Portable Architecture:** One binary-first workflow across ARM, x86, and RISC-V with swappable providers/channels/tools.
- 🔍 **Research Phase:** Proactive information gathering through tools before response generation — reduces hallucinations by fact-checking first.

### Why teams pick clawclawclaw

- **Lean by default:** small Rust binary, fast startup, low memory footprint.
- **Secure by design:** pairing, strict sandboxing, explicit allowlists, workspace scoping.
- **Fully swappable:** core systems are traits (providers, channels, tools, memory, tunnels).
- **No lock-in:** OpenAI-compatible provider support + pluggable custom endpoints.

## Quick Start

### Option 1: Homebrew (macOS/Linuxbrew)

```bash
brew install clawclawclaw
```

> Current executable name remains `clawclawclaw` for compatibility.

### Option 2: Clone + Bootstrap

```bash
git clone https://github.com/CSZHK/clawcalwclaw.git
cd clawcalwclaw
./bootstrap.sh
```

> **Note:** Source builds require ~2GB RAM and ~6GB disk. For resource-constrained systems, use `./bootstrap.sh --prefer-prebuilt` to download a pre-built binary instead.

### Option 3: Cargo Install

```bash
cargo install clawclawclaw

# Include full-screen terminal UI support in source installs
cargo install clawclawclaw --features tui-ratatui
```

### First Run

```bash
# Start the gateway (serves the Web Dashboard API/UI)
clawclawclaw gateway

# Open the dashboard URL shown in startup logs
# (default: http://127.0.0.1:3000/)

# Or chat directly
clawclawclaw chat "Hello!"

# Full-screen terminal UI (requires build with --features tui-ratatui)
clawclawclaw tui
```

For detailed setup options, see [docs/one-click-bootstrap.md](docs/one-click-bootstrap.md).

### Installation Docs (Canonical Source)

Use repository docs as the source of truth for install/setup instructions:

- [README Quick Start](#quick-start)
- [docs/one-click-bootstrap.md](docs/one-click-bootstrap.md)
- [docs/getting-started/README.md](docs/getting-started/README.md)

Issue comments can provide context, but they are not canonical installation documentation.

## Benchmark Snapshot (clawclawclaw vs OpenClaw, Reproducible)

Local machine quick benchmark (macOS arm64, Feb 2026) normalized for 0.8GHz edge hardware.

|                           | OpenClaw      | NanoBot        | PicoClaw        | clawclawclaw 🦞      |
| ------------------------- | ------------- | -------------- | --------------- | -------------------- |
| **Language**              | TypeScript    | Python         | Go              | **Rust**             |
| **RAM**                   | > 1GB         | > 100MB        | < 10MB          | **< 5MB**            |
| **Startup (0.8GHz core)** | > 500s        | > 30s          | < 1s            | **< 10ms**           |
| **Binary Size**           | ~28MB (dist)  | N/A (Scripts)  | ~8MB            | **~8.8 MB**          |
| **Cost**                  | Mac Mini $599 | Linux SBC ~$50 | Linux Board $10 | **Any hardware** |

> Notes: clawclawclaw results are measured on release builds using `/usr/bin/time -l`. OpenClaw requires Node.js runtime (typically ~390MB additional memory overhead), while NanoBot requires Python runtime. PicoClaw and clawclawclaw are static binaries. The RAM figures above are runtime memory; build-time compilation requirements are higher.

<p align="center">
  <img src="zero-claw.jpeg" alt="clawclawclaw vs OpenClaw Comparison" width="800" />
</p>

---

For full documentation, see [`docs/README.md`](docs/README.md) | [`docs/SUMMARY.md`](docs/SUMMARY.md)

## ⚠️ Official Repository & Impersonation Warning

**This is the only official clawclawclaw repository:**

> https://github.com/CSZHK/clawcalwclaw

Any other repository, organization, domain, or package claiming to be "clawclawclaw" or implying affiliation with clawclawclaw is **unauthorized and not affiliated with this project**. Known unauthorized forks will be listed in [TRADEMARK.md](TRADEMARK.md).

If you encounter impersonation or trademark misuse, please [open an issue](https://github.com/CSZHK/clawcalwclaw/issues).

---

## License

clawclawclaw is dual-licensed for maximum openness and contributor protection:

| License | Use case |
|---|---|
| [MIT](LICENSE-MIT) | Open-source, research, academic, personal use |
| [Apache 2.0](LICENSE-APACHE) | Patent protection, institutional, commercial deployment |

You may choose either license. **Contributors automatically grant rights under both** — see [CLA.md](CLA.md) for the full contributor agreement.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CLA.md](CLA.md). Implement a trait, submit a PR.

---

**clawclawclaw** — Zero overhead. Zero compromise. Deploy anywhere. Swap anything. 🦞

## Star History

<p align="center">
  <a href="https://www.star-history.com/#CSZHK/clawcalwclaw&type=date&legend=top-left">
    <picture>
     <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=CSZHK/clawcalwclaw&type=date&theme=dark&legend=top-left" />
     <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=CSZHK/clawcalwclaw&type=date&legend=top-left" />
     <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=CSZHK/clawcalwclaw&type=date&legend=top-left" />
    </picture>
  </a>
</p>
