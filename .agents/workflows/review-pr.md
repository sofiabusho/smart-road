---
description: Review a pull request for smart-road against project gates, ticket scope, and code quality.
---

# PR Review Playbook: smart-road

You are reviewing a pull request for **smart-road**. Produce a structured verdict: **Approve**, **Request changes**, or **Block**, with evidence for every finding.

This project is **requirements-first** and **audit-first**. A PR is not merge-ready unless it satisfies ticket scope, cross-track ownership rules, automated quality gates, and relevant **REQ-*** / **AUD-*** traceability.

---

## 0. Inputs

Collect before reviewing:

| Input | How to get it |
|-------|----------------|
| PR number or branch | User provides `gh pr view <N>`, branch name, or local branch |
| Ticket ID | Branch name (`feat/A04-arrow-spawn`), PR title, or `docs/pr-messages/{ID}-*-pr.md` |
| PR message artifact | `docs/pr-messages/{TicketID}-*-pr.md` (required for ticket PRs) |
| Diff scope | `git diff main...HEAD` or `gh pr diff <N>` |

If the PR message artifact is missing, flag it as a **blocking** finding.

---

## 1. Read First (do not skip)

| Document | Why |
|----------|-----|
| `AGENTS.md` | Stack constraints, coding standards, PR checklist |
| `docs/ticket-tracker.md` | Ticket scope, deps, REQ/AUD coverage, status |
| `docs/SDS.md` §13 | Cross-track file ownership and API contracts |
| `docs/audit.md` | Acceptance gates tied to this ticket |
| `docs/requirements.md` | REQ IDs claimed in the PR message |
| `docs/pr-messages/{TicketID}-*-pr.md` | Author's evidence and traceability claims |

---

## 2. Automated Checks (run locally)

From the repo root, on the PR branch:

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo build
```

Record pass/fail for each. Any failure is **blocking** unless the PR explicitly documents a known, approved exception.

Optional when the ticket touches rendering or manual AUD items:

```bash
cargo run   # confirm window launches; note visual observations
```

---

## 3. Scope and Traceability Review

### 3.1 Ticket alignment

- [ ] Branch name matches one ticket (`feat/A##-*`, `feat/B##-*`, `feat/C##-*`, or `fix/A##-*`).
- [ ] Changes implement **only** that ticket's scope — no unrelated refactors or drive-by fixes.
- [ ] Cross-track deps (🔗 in tracker) are ✅ before this ticket could legitimately merge.
- [ ] `docs/ticket-tracker.md` updated: ticket status, coverage matrices if applicable.

### 3.2 REQ / AUD traceability

For every REQ and AUD ID listed on the ticket row and in the PR message:

- [ ] Code or tests actually satisfy the requirement — not just mentioned in prose.
- [ ] Manual AUD items have verification notes (how observed, not just "Pass").
- [ ] No mandatory AUD item marked Pass without evidence.

If the PR claims REQ/AUD coverage that the diff does not support, flag as **blocking**.

### 3.3 PR message artifact

Against `docs/pr-messages/pr-template.md`:

- [ ] File exists at `docs/pr-messages/{TicketID}-*-pr.md`.
- [ ] Summary, key changes, technical decisions, and verification sections are filled in.
- [ ] Automated check results match what you observed when re-running commands.
- [ ] Next steps identify the logical follow-on ticket.

---

## 4. Cross-Track and Architecture Review

Per `docs/SDS.md` §13.1:

| Module / file | Owner | Review question |
|---------------|-------|-----------------|
| `main.rs`, `app.rs` | A | Did a non-A track edit without SDS update? |
| `config.rs` | A | Are new tunables in the right place? |
| `intersection.rs` | A + B | A: topology/layout; B: `path` polylines only |
| `spawn.rs`, `input.rs`, `render.rs` | A | No simulation or smart logic leaked in? |
| `vehicle.rs` | B | No render/input/smart coupling? |
| `smart.rs`, `stats*.rs` | C | No presentation-layer edits? |

**Blocking** if another track edits owned files without updating SDS §13 and calling it out in the PR message.

### Domain rules (from `AGENTS.md`)

- Smart controller owns velocity commands inside the managed zone.
- Vehicles adhere to precomputed lane polylines — no mid-path lane changes.
- Crossing timer starts at smart-system detection, not spawn.
- Render reads snapshots only — no mutation of simulation state.
- No unapproved dependencies or game engines.
- No `println!` in hot paths.

### API contract drift

If the PR changes public types or function signatures exported in SDS §13.2–13.4:

- [ ] SDS updated to match.
- [ ] Dependent tracks would not break silently.

---

## 5. Code Quality Review

Review the diff for:

- **Correctness** — logic bugs, off-by-one, missing edge cases, resource leaks (SDL textures/surfaces).
- **Rust idioms** — unnecessary clones, ignored `Result`, unsafe without justification.
- **Module boundaries** — focused modules; no god-files.
- **Naming** — matches `AGENTS.md` conventions (`snake_case`, `PascalCase`, `SCREAMING_SNAKE_CASE`).
- **Tests** — new logic has unit or integration tests where applicable; smoke tests stay headless (no SDL2) when possible.
- **Docs** — README/SDS updated when public behavior or setup changes.

Classify findings:

| Severity | Meaning |
|----------|---------|
| **Blocker** | Must fix before merge (failing tests, wrong REQ/AUD claim, cross-track violation, audit regression) |
| **Major** | Should fix before merge (missing tests for core logic, API drift, missing PR artifact) |
| **Minor** | Nice to fix (style, naming, comment clarity) |
| **Nit** | Optional polish |

---

## 6. Output Format

Return this structure:

```markdown
# PR Review: {ticket-id} — {short title}

**Verdict**: Approve | Request changes | Block
**Branch**: `{branch-name}`
**Ticket**: {TicketID}
**PR message**: `docs/pr-messages/{file}` (found | missing)

## Automated checks

| Command | Result |
|---------|--------|
| `cargo test` | pass / fail |
| `cargo clippy -- -D warnings` | pass / fail |
| `cargo fmt --check` | pass / fail |
| `cargo build` | pass / fail |

## Traceability

| ID | Claimed | Verified | Notes |
|----|---------|----------|-------|
| REQ-* | yes/no | yes/no | … |
| AUD-* | yes/no | yes/no | … |

## Findings

| Severity | Location | Finding |
|----------|----------|---------|
| Blocker | `path:line` | … |

(If no issues: "No blockers or majors found.")

## Checklist summary

- [ ] Ticket scope respected
- [ ] Cross-track ownership respected
- [ ] PR message artifact present and accurate
- [ ] Tracker updated
- [ ] Tests adequate
- [ ] Audit items verified

## Merge recommendation

One paragraph: merge now, merge after fixes, or do not merge — with the single most important reason.
```

Sort findings by severity (Blocker → Major → Minor → Nit).

---

## 7. Optional: Code-level diff review via Bugbot

For deeper bug-hunting on the diff, launch the Bugbot subagent **after** completing sections 1–6:

```text
Full Repository Path: /home/someuan/smart-road
Diff: branch changes
Custom Instructions: Review against AGENTS.md and docs/SDS.md §13. Flag cross-track file edits, SDL2 resource handling issues, and simulation correctness bugs. Ignore style nits already caught by clippy.
```

Merge Bugbot findings into section 6's table; deduplicate.

---

## 8. What Not to Do

- Do not approve a PR with failing `cargo test` or `cargo clippy -- -D warnings`.
- Do not approve without verifying AUD items listed on the ticket.
- Do not suggest scope expansion — open a new tracker row instead.
- Do not edit `docs/requirements.md` or `docs/audit.md` during review unless the user explicitly requests it.
- Do not fix code unless the user asks you to address findings.
