# OPTIONAL EXAMPLE — Not loaded by default

<!--
  Copy to .agents/rules/rtk-shell.md if your team uses a token-saving CLI wrapper.
  Derived from mini-framework antigravity-rtk-rules — generalized.
-->

```markdown
---
description: Prefix shell commands with rtk to reduce token usage from command output
trigger: always_on
---

# Shell Command Wrapper (RTK)

When running shell commands in the agent context, prefix with `rtk` if available:

\`\`\`bash
rtk git status
rtk npm test
rtk find . -name "*.ts"
\`\`\`

## Meta commands

\`\`\`bash
rtk gain              # Show token savings
rtk discover          # Find missed RTK opportunities
rtk proxy <cmd>       # Run raw command (debugging)
\`\`\`

## When to skip

- Interactive commands requiring TTY input
- When `rtk` is not installed (fall back to raw commands)
- When full unfiltered output is explicitly needed for debugging
```
