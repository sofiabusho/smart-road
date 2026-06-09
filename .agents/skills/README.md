# Project Skills

<!-- Agent skills are reusable instruction packs for specialized behaviors.
     Cursor discovers skills from SKILL.md files in this directory (or via .cursor/skills/).
     This starter kit does not ship production skills — add your own per project. -->

## What Belongs Here

A **skill** is a folder (or single file) with a `SKILL.md` that tells the agent:

- When to activate (triggers, keywords, slash commands)
- What procedure to follow
- What outputs to produce

Good candidates for project skills:

- Domain-specific review checklists (security, accessibility, API versioning)
- Deployment or release procedures
- Repeated codegen patterns (e.g., "scaffold a new service module")
- Integration test harness setup

## How to Add a Skill

1. Create a directory: `.agents/skills/my-skill/`
2. Add `SKILL.md` with YAML frontmatter:

```markdown
---
name: my-skill
description: Short description shown in skill pickers. Use when the user asks for X.
---

# My Skill

Step-by-step instructions for the agent...
```

3. Optionally add a `README.md` for humans (usage examples, slash commands).
4. Reference the skill from `AGENTS.md` or `docs/AGENT_WORKFLOW.md` if agents should use it routinely.

## Cursor Integration

Cursor also supports skills under `.cursor/skills/` (user- or project-scoped). Choose one location and stay consistent:

| Location | Scope |
|----------|-------|
| `.agents/skills/` | Project repo (portable with this starter kit) |
| `.cursor/skills/` | Cursor-specific project skills |

## Optional Examples

See `examples/optional-skills/` in the starter kit root for **commented reference patterns** (not loaded by default). Copy and adapt — do not enable verbatim unless the behavior fits your project.

## What Not to Put Here

- One-off ticket instructions → use `docs/ticket-tracker.md` and `.agents/workflows/`
- Permanent coding standards → use `AGENTS.md` or `.cursor/rules/`
- Acceptance criteria → use `docs/audit.md`
