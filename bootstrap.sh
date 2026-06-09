#!/usr/bin/env bash
# Copy agent-starter-kit into a target project directory and list placeholders to edit.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${1:-}"

if [[ -z "$TARGET" ]]; then
  echo "Usage: $0 <target-project-directory>"
  echo ""
  echo "Example:"
  echo "  $0 ~/projects/my-new-app"
  exit 1
fi

mkdir -p "$TARGET"

# Copy kit contents (including dot-directories)
shopt -s dotglob
cp -r "$SCRIPT_DIR"/* "$TARGET/"
shopt -u dotglob

# Do not copy bootstrap.sh into target (optional: user may want it — we skip to avoid clutter)
rm -f "$TARGET/bootstrap.sh"

echo "✓ Agent starter kit copied to: $TARGET"
echo ""
echo "Recommended fill order:"
echo "  1. docs/requirements.md   (REQ IDs)"
echo "  2. docs/audit.md          (AUD IDs)"
echo "  3. docs/PRD.md + docs/SDS.md"
echo "  4. AGENTS.md              (stack, commands, layout)"
echo "  5. docs/ticket-tracker.md"
echo ""
echo "Placeholders still to edit ({{...}}):"
if command -v rg &>/dev/null; then
  rg -n '\{\{[A-Z0-9_]+\}\}' "$TARGET" --glob '!examples/**' || true
else
  grep -rn '{{' "$TARGET" --include='*.md' --include='*.mdc' --exclude-dir=examples 2>/dev/null || true
fi
echo ""
echo "See common-for-projects/README.md for the full mapping table and workflow guide."
