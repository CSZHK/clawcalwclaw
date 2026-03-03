# docs/CLAUDE.md — Documentation System

Scope: all files under `docs/`. Read in conjunction with root [`CLAUDE.md`](../CLAUDE.md).

---

## 4.1 Documentation System Contract (Required)

Treat documentation as a first-class product surface, not a post-merge artifact.

Canonical entry points:

- repository landing + localized hubs: `README.md`, `docs/i18n/zh-CN/README.md`, `docs/i18n/ja/README.md`, `docs/i18n/ru/README.md`, `docs/i18n/fr/README.md`, `docs/i18n/vi/README.md`, `docs/i18n/el/README.md`
- docs hubs: `docs/README.md`, `docs/i18n/zh-CN/README.md`, `docs/i18n/ja/README.md`, `docs/i18n/ru/README.md`, `docs/i18n/fr/README.md`, `docs/i18n/vi/README.md`, `docs/i18n/el/README.md`
- unified TOC: `docs/SUMMARY.md`
- i18n governance docs: `docs/i18n-guide.md`, `docs/i18n/README.md`, `docs/i18n-coverage.md`

Supported locales (current contract):

- `en`, `zh-CN`, `ja`, `ru`, `fr`, `vi`, `el`

Collection indexes (category navigation):

- `docs/getting-started/README.md`
- `docs/reference/README.md`
- `docs/operations/README.md`
- `docs/security/README.md`
- `docs/hardware/README.md`
- `docs/contributing/README.md`
- `docs/project/README.md`

Runtime-contract references (must track behavior changes):

- `docs/commands-reference.md`
- `docs/providers-reference.md`
- `docs/channels-reference.md`
- `docs/config-reference.md`
- `docs/operations-runbook.md`
- `docs/troubleshooting.md`
- `docs/one-click-bootstrap.md`

Required docs governance rules:

- Keep README/hub top navigation and quick routes intuitive and non-duplicative.
- Keep entry-point parity across all supported locales (`en`, `zh-CN`, `ja`, `ru`, `fr`, `vi`, `el`) when changing navigation architecture.
- If a change touches docs IA, runtime-contract references, or user-facing wording in shared docs, perform i18n follow-through for currently supported locales in the same PR:
  - Update locale navigation links (`README*`, `docs/README*`, `docs/SUMMARY.md`).
  - Update canonical locale hubs and summaries under `docs/i18n/<locale>/` for every supported locale.
  - Update localized runtime-contract docs where equivalents exist (currently full trees for `vi` and `el`; do not regress `zh-CN`/`ja`/`ru`/`fr` hub parity).
  - Keep `docs/*.<locale>.md` compatibility shims aligned if present.
- Follow `docs/i18n-guide.md` as the mandatory completion checklist when docs navigation or shared wording changes.
- Keep proposal/roadmap docs explicitly labeled; avoid mixing proposal text into runtime-contract docs.
- Keep project snapshots date-stamped and immutable once superseded by a newer date.

### 4.2 Docs i18n Completion Gate (Required)

For any PR that changes docs IA, locale navigation, or shared docs wording:

1. Complete i18n follow-through in the same PR using `docs/i18n-guide.md`.
2. Keep all supported locale hubs/summaries navigable through canonical `docs/i18n/<locale>/` paths.
3. Update `docs/i18n-coverage.md` when coverage status or locale topology changes.
4. If any translation must be deferred, record explicit owner + follow-up issue/PR in the PR description.

---

## 7.6 Docs System / README / IA Changes

- Treat docs navigation as product UX: preserve clear pathing from README -> docs hub -> SUMMARY -> category index.
- Keep top-level nav concise; avoid duplicative links across adjacent nav blocks.
- When runtime surfaces change, update related references (`commands/providers/channels/config/runbook/troubleshooting`).
- Keep multilingual entry-point parity for all supported locales (`en`, `zh-CN`, `ja`, `ru`, `fr`, `vi`, `el`) when nav or key wording changes.
- When shared docs wording changes, sync corresponding localized docs for supported locales in the same PR (or explicitly document deferral and follow-up PR).
- Treat `docs/i18n/<locale>/**` as canonical for localized hubs/summaries; keep docs-root compatibility shims aligned when edited.
- Apply `docs/i18n-guide.md` completion checklist before merge and include i18n status in PR notes.
- For docs snapshots, add new date-stamped files for new sprints rather than rewriting historical context.
