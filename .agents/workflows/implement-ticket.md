---
description: Implement a specific ticket for smart-road.
---

# Ticket Implementation Playbook: {Ticket}

You are implementing a single ticket for **smart-road**. Produce maintainable code that strictly follows project standards and passes the audit gate before handover.

---

## 1. Primary Directives

Before writing code, read:

| Document | Why |
|----------|-----|
| `AGENTS.md` | Stack, conventions, directory layout, commands |
| `docs/ticket-tracker.md` | Ticket scope, dependencies, REQ/AUD IDs, status |
| `docs/SDS.md` & `docs/PRD.md` | Technical and product detail for this area |
| `docs/audit.md` & `docs/requirements.md` | Verification gates and stakeholder requirements |
| `docs/AGENT_WORKFLOW.md` | How docs connect and how to close tickets |

**Verification gate**: Work is not finished until every AUD ID listed on this ticket is verified and tests pass.

---

## 2. Technical Standards

Follow `AGENTS.md` for language, framework, and style rules. In summary:

- Match existing patterns in the codebase before introducing new abstractions.
- Keep functions focused; extract helpers when logic grows unwieldy.
- Respect technology constraints (Rust, SDL2, no unapproved game engines).
- Smart controller owns velocity commands inside the intersection zone.

---

## 3. Workflow Steps

### Step 1: Analysis

- Find the ticket in `docs/ticket-tracker.md` — confirm **track (A/B/C)**, status, dependencies, size, and coverage (REQ/AUD IDs).
- **Confirm cross-track deps** — every 🔗 ticket in the Deps column must be ✅ before starting (e.g., `B01` waits for `A04`).
- Read `docs/SDS.md` §13 for your track's exports and file ownership.
- Read the relevant SDS sections for API contracts, data models, or critical structures.
- If dependencies are not ✅ Done, stop and pick an unblocked ticket on your track (or document the blocker).

### Step 2: Implementation

- Implement only what the ticket describes; split scope creep into a new tracker row.
- Follow naming, module, and error-handling conventions from `AGENTS.md`.
- Update user-facing or API documentation if behavior or public surface changes.

### Step 3: Testing and QA

- Run `cargo test` — all existing and new tests must pass.
- Run `cargo clippy -- -D warnings` and `cargo fmt --check` — fix lint/format issues.
- **Bug workflow** (if you find a regression):
  1. Add a minimal failing test that reproduces the bug.
  2. Fix the implementation.
  3. Re-run the full test suite.

### Step 4: Audit Verification

- Walk through each **AUD-*** ID listed on the ticket in `docs/audit.md`.
- Record pass/fail and notes in the PR message (Step 5).
- Do not mark the ticket Done if any mandatory AUD item fails.

### Step 5: Documentation and Handover

1. **PR message** — use `docs/pr-messages/pr-template.md` as the blueprint.
2. **Save artifact** — `docs/pr-messages/{TicketID}-{short-desc}-pr.md` (e.g. `A04-arrow-spawn-pr.md`, `C03-yield-pr.md`).
3. **Update tracker** — set ticket status to ✅ Done in `docs/ticket-tracker.md`.
4. **Refresh matrices** — update Requirements/Audit coverage tables if this ticket closes a gap.

---

## 4. Ticket Context: {Ticket}

> [!IMPORTANT]
> Paste the full ticket row (ID, description, deps, REQ/AUD coverage) from `docs/ticket-tracker.md` here before executing.

| Field | Value |
|-------|-------|
| Ticket ID | {Ticket} |
| Dependencies | {{DEPS}} |
| REQ coverage | {{REQ_IDS}} |
| AUD coverage | {{AUD_IDS}} |

**Begin implementation.** Focus on correct behavior, clear abstractions, and audit-ready handover.
