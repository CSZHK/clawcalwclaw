# .github/CLAUDE.md — PR / CI / Collaboration

Scope: GitHub workflows, PR processes, and contributor discipline. Read in conjunction with root [`CLAUDE.md`](../CLAUDE.md).

---

## 6.1 Branch / Commit / PR Flow (Required)

All contributors (human or agent) must follow the same collaboration flow:

- Create and work from a non-`main` branch.
- Commit changes to that branch with clear, scoped commit messages.
- Open a PR to `main` by default (`dev` is optional for integration batching); do not push directly to `dev` or `main`.
- `main` accepts direct PR merges after required checks and review policy pass.
- Wait for required checks and review outcomes before merging.
- Merge via PR controls (squash/rebase/merge as repository policy allows).
- After merge/close, clean up task branches/worktrees that are no longer needed.
- Keep long-lived branches only when intentionally maintained with clear owner and purpose.

### 6.1A PR Disposition and Workflow Authority (Required)

- Decide merge/close outcomes from repository-local authority in this order: `.github/workflows/**`, GitHub branch protection/rulesets, `docs/pr-workflow.md`, then root `CLAUDE.md`.
- External agent skills/templates are execution aids only; they must not override repository-local policy.
- A normal contributor PR targeting `main` is valid under the main-first flow when required checks and review policy are satisfied; use `dev` only for explicit integration batching.
- Direct-close the PR (do not supersede/replay) when high-confidence integrity-risk signals exist:
  - unapproved or unrelated repository rebranding attempts (for example replacing project logo/identity assets)
  - unauthorized platform-surface expansion (for example introducing `web` apps, dashboards, frontend stacks, or UI surfaces not requested by maintainers)
  - title/scope deception that hides high-risk code changes (for example `docs:` title with broad `src/**` changes)
  - spam-like or intentionally harmful payload patterns
  - multi-domain dirty-bundle changes with no safe, auditable isolation path
- If unauthorized platform-surface expansion is detected during review/implementation, report to maintainers immediately and pause further execution until explicit direction is given.
- Use supersede flow only when maintainers explicitly want to preserve valid work and attribution.
- In public PR close/block comments, state only direct actionable reasons; do not include internal decision-process narration or "non-reason" qualifiers.

### 6.1B Assignee-First Gate (Required)

- For any GitHub issue or PR selected for active handling, the first action is to ensure `@chumyin` is an assignee.
- This is additive ownership: keep existing assignees and add `@chumyin` if missing.
- Do not start triage/review/implementation/merge work before assignee assignment is confirmed.
- Queue safety rule: assign only the currently active target; do not pre-assign future queued targets.

---

## 9) Collaboration and PR Discipline

- Follow `.github/pull_request_template.md` fully (including side effects / blast radius).
- Keep PR descriptions concrete: problem, change, non-goals, risk, rollback.
- For issue-driven work, add explicit issue-closing keywords in the **PR body** for every resolved issue (for example `Closes #1502`).
- Do not rely on issue comments alone for linkage visibility; comments are supplemental, not a substitute for PR-body closing references.
- Default to one issue per clean commit/PR track. For multiple issues, split into separate clean commits/PRs unless there is clear technical coupling.
- If multiple issues are intentionally bundled in one PR, document the coupling rationale explicitly in the PR summary.
- Commit hygiene is mandatory: stage only task-scoped files and split unrelated changes into separate commits/worktrees.
- Completion hygiene is mandatory: after merge/close, clean stale local branches/worktrees before starting the next track.
- Use conventional commit titles.
- Prefer small PRs (`size: XS/S/M`) when possible.
- Agent-assisted PRs are welcome, **but contributors remain accountable for understanding what their code will do**.

### 9.1 Privacy/Sensitive Data and Neutral Wording (Required)

Treat privacy and neutrality as merge gates, not best-effort guidelines.

- Never commit personal or sensitive data in code, docs, tests, fixtures, snapshots, logs, examples, or commit messages.
- Prohibited data includes (non-exhaustive): real names, personal emails, phone numbers, addresses, access tokens, API keys, credentials, IDs, and private URLs.
- Use neutral project-scoped placeholders (for example: `user_a`, `test_user`, `project_bot`, `example.com`) instead of real identity data.
- Test names/messages/fixtures must be impersonal and system-focused; avoid first-person or identity-specific language.
- If identity-like context is unavoidable, use ZeroClaw-scoped roles/labels only (for example: `ZeroClawAgent`, `ZeroClawOperator`, `zeroclaw_user`) and avoid real-world personas.
- Recommended identity-safe naming palette (use when identity-like context is required):
    - actor labels: `ZeroClawAgent`, `ZeroClawOperator`, `ZeroClawMaintainer`, `zeroclaw_user`
    - service/runtime labels: `zeroclaw_bot`, `zeroclaw_service`, `zeroclaw_runtime`, `zeroclaw_node`
    - environment labels: `zeroclaw_project`, `zeroclaw_workspace`, `zeroclaw_channel`
- If reproducing external incidents, redact and anonymize all payloads before committing.
- Before push, review `git diff --cached` specifically for accidental sensitive strings and identity leakage.

### 9.2 Superseded-PR Attribution (Required)

When a PR supersedes another contributor's PR and carries forward substantive code or design decisions, preserve authorship explicitly.

- In the integrating commit message, add one `Co-authored-by: Name <email>` trailer per superseded contributor whose work is materially incorporated.
- Use a GitHub-recognized email (`<login@users.noreply.github.com>` or the contributor's verified commit email) so attribution is rendered correctly.
- Keep trailers on their own lines after a blank line at commit-message end; never encode them as escaped `\\n` text.
- In the PR body, list superseded PR links and briefly state what was incorporated from each.
- If no actual code/design was incorporated (only inspiration), do not use `Co-authored-by`; give credit in PR notes instead.

### 9.3 Superseded-PR PR Template (Recommended)

When superseding multiple PRs, use a consistent title/body structure to reduce reviewer ambiguity.

- Recommended title format: `feat(<scope>): unify and supersede #<pr_a>, #<pr_b> [and #<pr_n>]`
- If this is docs/chore/meta only, keep the same supersede suffix and use the appropriate conventional-commit type.
- In the PR body, include the following template (fill placeholders, remove non-applicable lines):

```md
## Supersedes
- #<pr_a> by @<author_a>
- #<pr_b> by @<author_b>
- #<pr_n> by @<author_n>

## Integrated Scope
- From #<pr_a>: <what was materially incorporated>
- From #<pr_b>: <what was materially incorporated>
- From #<pr_n>: <what was materially incorporated>

## Attribution
- Co-authored-by trailers added for materially incorporated contributors: Yes/No
- If No, explain why (for example: no direct code/design carry-over)

## Non-goals
- <explicitly list what was not carried over>

## Risk and Rollback
- Risk: <summary>
- Rollback: <revert commit/PR strategy>
```

### 9.4 Superseded-PR Commit Template (Recommended)

When a commit unifies or supersedes prior PR work, use a deterministic commit message layout so attribution is machine-parsed and reviewer-friendly.

- Keep one blank line between message sections, and exactly one blank line before trailer lines.
- Keep each trailer on its own line; do not wrap, indent, or encode as escaped `\n` text.
- Add one `Co-authored-by` trailer per materially incorporated contributor, using GitHub-recognized email.
- If no direct code/design is carried over, omit `Co-authored-by` and explain attribution in the PR body instead.

```text
feat(<scope>): unify and supersede #<pr_a>, #<pr_b> [and #<pr_n>]

<one-paragraph summary of integrated outcome>

Supersedes:
- #<pr_a> by @<author_a>
- #<pr_b> by @<author_b>
- #<pr_n> by @<author_n>

Integrated scope:
- <subsystem_or_feature_a>: from #<pr_x>
- <subsystem_or_feature_b>: from #<pr_y>

Co-authored-by: <Name A> <login_a@users.noreply.github.com>
Co-authored-by: <Name B> <login_b@users.noreply.github.com>
```

---

## Reference Docs

- `CONTRIBUTING.md`
- `docs/README.md`
- `docs/SUMMARY.md`
- `docs/i18n-guide.md`
- `docs/i18n/README.md`
- `docs/i18n-coverage.md`
- `docs/docs-inventory.md`
- `docs/commands-reference.md`
- `docs/providers-reference.md`
- `docs/channels-reference.md`
- `docs/config-reference.md`
- `docs/operations-runbook.md`
- `docs/troubleshooting.md`
- `docs/one-click-bootstrap.md`
- `docs/pr-workflow.md`
- `docs/reviewer-playbook.md`
- `docs/ci-map.md`
- `docs/actions-source-policy.md`
