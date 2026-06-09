---
title: "feat({{AREA}}): {{TICKET_NAME}}"
---

# PR Implementation Report: {{TICKET_ID}}

<!-- Save completed reports as: docs/pr-messages/{TicketID}-{short-desc}-pr.md
     Example: docs/pr-messages/T10-auth-middleware-pr.md -->

## Summary

<!-- 2–4 sentences: what changed and which REQ/AUD IDs this satisfies. -->

{{BRIEF_EXECUTIVE_SUMMARY}}

## Key Changes

- **{{MODULE_OR_FILE}}**: {{CHANGE_DESCRIPTION}}
- **{{MODULE_OR_FILE}}**: {{CHANGE_DESCRIPTION}}

## Technical Decisions

<!-- Document non-obvious choices so the next agent or reviewer understands "why." -->

- **{{DECISION_TITLE}}**: {{RATIONALE}}
- **{{DECISION_TITLE}}**: {{RATIONALE}}

## Verification Results

### Automated Checks

- [ ] `{{TEST_COMMAND}}` passes (including new tests for this feature)
- [ ] `{{LINT_COMMAND}}` passes
- [ ] Build succeeds (if applicable): `{{BUILD_COMMAND}}`

### Manual Audit (against `docs/audit.md`)

<!-- List every AUD ID tied to this ticket. Mark Pass / Fail / N/A. -->

- [ ] **AUD-{{N}}**: {{Pass|Fail|N/A}} — {{NOTES}}
- [ ] **AUD-{{N}}**: {{Pass|Fail|N/A}} — {{NOTES}}

### Requirements Traceability

- [ ] **REQ-{{N}}**: {{HOW_THIS_TICKET_SATISFIES_IT}}

## Artifacts

- **Test output**: {{link, log snippet, or "see CI run #N"}}
- **Lint output**: {{link or "clean"}}

---

## Next Steps

<!-- What ticket or gate comes next; any follow-up debt. -->

{{NEXT_TICKET_OR_GATE}}
