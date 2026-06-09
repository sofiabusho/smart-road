# Agent Workflow — smart-road

<!-- How coding agents should navigate docs, pick up work, and close tickets.
     This is the "map" — AGENTS.md is the "rules of the road." -->

---

## 1. Document Hierarchy

Read documents in this order when starting any task:

```text
AGENTS.md                    ← coding rules, stack, directory layout
    ↓
docs/requirements.md         ← what stakeholders require (REQ IDs)
docs/audit.md                ← how "done" is verified (AUD IDs)
    ↓
docs/PRD.md                  ← product detail and priorities
docs/SDS.md                  ← technical specs and API contracts
    ↓
docs/ticket-tracker.md       ← current sprint, ticket status, dependencies
    ↓
docs/pr-messages/pr-template.md  ← handover format when closing a ticket
```

**Rule**: Never mark a ticket done without verifying relevant **AUD-*** items in `docs/audit.md`.

---

## 2. Starting a New Ticket

1. **Declare your track** — A (platform), B (simulation), or C (smart/stats). Pick tickets only from that track unless assigned an explicit integration ticket (e.g. C07).
2. **Locate the ticket** in `docs/ticket-tracker.md` — note ID, dependencies, REQ/AUD coverage, and 🔗 cross-track deps.
3. **Read source of truth** — `AGENTS.md`, `docs/SDS.md` §13 (interfaces), relevant PRD/SDS sections, linked REQ and AUD IDs.
4. **Check dependencies** — intra-track and 🔗 cross-track prerequisite tickets must be ✅ Done (or explicitly waived).
5. **Update status** — set ticket to 🟢 In Progress in `docs/ticket-tracker.md`.
6. **Run the workflow** — follow `.agents/workflows/implement-ticket.md`.

### Picking work by track

| Track | Start here (after A01 ✅) | While blocked |
|-------|---------------------------|---------------|
| **A** | A02 ∥ A03 → A04 → A05/A06 → A07 (after B02) | — |
| **B** | B01 (after A04) → B02 (after A03) → B03/B04 | Read SDS §13; draft types in branch only |
| **C** | C01 (after B02) → C02 → C03/C04 → C05/C06 → C07 | Read SDS §13; sketch interfaces |

Do not implement another track's owned modules — coordinate via SDS §13 stubs and PR review.

---

## 3. During Implementation

| Activity | Reference |
|----------|-----------|
| Architecture decisions | `docs/SDS.md`, `docs/PRD.md` |
| Coding style | `AGENTS.md` |
| API contracts | `docs/SDS.md` |
| Acceptance criteria | `docs/audit.md` (AUD IDs on the ticket) |
| Scope boundaries | `docs/requirements.md`, PRD non-goals |

**Do not**:

- Expand scope beyond the ticket without updating `docs/ticket-tracker.md` and requirements.
- Skip tests or audit checks to "move faster."
- Edit `docs/requirements.md` or `docs/audit.md` without stakeholder approval (they are gates, not scratch pads).

---

## 4. Closing a Ticket

1. **Run quality commands** — tests, lint, build (see `AGENTS.md` Development Workflow).
2. **Verify audit items** — every AUD ID listed on the ticket row in the tracker.
3. **Write PR message** — copy `docs/pr-messages/pr-template.md`; save as `docs/pr-messages/{TicketID}-{short-desc}-pr.md`.
4. **Update tracker** — set ticket status to ✅ Done; refresh coverage matrices if needed.
5. **Update docs** — if public API or behavior changed, update SDS and/or user-facing README.

---

## 5. Traceability Flow

```text
requirements.md (REQ-*)  ──→  PRD.md (detailed spec)
        │                           │
        └──────────→  ticket-tracker.md (tickets + coverage)
                                │
audit.md (AUD-*)  ──────────────┘
                                │
                                ↓
                    pr-messages/{ticket}-pr.md (evidence)
```

Every ticket should list which **REQ-*** and **AUD-*** IDs it satisfies. Gates (G1, G2, …) aggregate ticket completion.

---

## 6. When Stuck

| Situation | Action |
|-----------|--------|
| Spec ambiguity | Check PRD → SDS → requirements; if still unclear, note assumption in PR message |
| Audit failure | Fix before marking Done; do not waive AUD items without documenting in tracker |
| Blocked dependency | Set ticket 🔴 Blocked; note blocker in tracker; pick next unblocked ticket |
| Scope creep | Split into new ticket; add to tracker with new REQ linkage |

---

## 7. Optional Extensions

| Path | Purpose |
|------|---------|
| `.agents/workflows/implement-ticket.md` | Step-by-step implementation playbook |
| `.agents/skills/` | Project-specific agent skills (see README there) |
| `.agents/rules/` | Supplemental rules beyond `AGENTS.md` |
| `.cursor/rules/project-standards.mdc` | Cursor IDE rule pointing agents to this doc set |
| `examples/` | Optional reference patterns (not loaded by default) |
