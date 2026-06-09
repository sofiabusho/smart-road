# Project Rules

<!-- Supplemental agent rules beyond AGENTS.md and .cursor/rules/.
     Use for optional behaviors, tool-specific guidance, or team preferences. -->

## What Belongs Here

**Rules** are short, persistent instructions the agent should follow. Unlike skills (procedures), rules are constraints or defaults.

| Location | Format | Loaded by |
|----------|--------|-----------|
| `.cursor/rules/*.mdc` | YAML frontmatter + markdown | Cursor IDE (always or per-glob) |
| `.agents/rules/*.md` | YAML frontmatter + markdown | Agent tooling that reads this folder |
| `AGENTS.md` | Markdown | All agents — primary coding standards |

This directory is for **optional supplements**. Core standards belong in `AGENTS.md`.

## How to Add a Rule

1. Create a file: `.agents/rules/my-rule.md`
2. Add frontmatter describing when it applies:

```markdown
---
description: When to use RTK-wrapped shell commands
trigger: always_on
---

# My Rule

Rule content...
```

3. For Cursor-native rules, prefer `.cursor/rules/` with `.mdc` extension (see `project-standards.mdc`).

### Cursor `.mdc` Frontmatter Fields

| Field | Purpose |
|-------|---------|
| `description` | Shown in rule picker |
| `globs` | Apply when matching files are open (e.g., `**/*.ts`) |
| `alwaysApply` | `true` = every session |

## Optional Examples

See `examples/optional-rules/` for **commented reference patterns** (e.g., token-saving CLI wrappers, response-style preferences). Copy only what your team wants — examples are not active in the starter kit.

## What Not to Put Here

- Ticket workflows → `.agents/workflows/`
- Requirements or audit gates → `docs/requirements.md`, `docs/audit.md`
- Lengthy architecture → `docs/SDS.md`
