# OPTIONAL EXAMPLE — Not loaded by default

<!--
  Copy to .agents/skills/caveman/SKILL.md if your team wants terse agent responses.
  Derived from mini-framework caveman skill — generalized, no project coupling.
-->

```markdown
---
name: caveman
description: Compress responses to terse prose. Use when user invokes /caveman or asks for minimal tokens.
---

# Caveman Response Style

When active, compress every response:

- Drop articles (a/an/the), filler, pleasantries, hedging
- Keep all technical terms, code, errors, and symbols exact
- Fragments OK; pattern: [thing] [action] [reason]

## Modes

| Command | Effect |
|---------|--------|
| `/caveman lite` | Drop filler only; full sentences |
| `/caveman` | Default compression |
| `/caveman ultra` | Maximum compression |
| `stop caveman` | Return to normal prose |

## Auto-clarity

Switch to normal prose for: security warnings, irreversible actions, user confusion.
Resume caveman after the clear section.

## Boundaries

Code blocks, commit messages, and PR text stay in normal professional prose unless user requests otherwise.
```
