#!/bin/bash
#
# Uninstall git hooks
#
# Usage: ./.githooks/uninstall.sh
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GIT_HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo -e "${BLUE}Uninstalling git hooks...${NC}"
echo ""

# List of hooks to uninstall
HOOKS=("pre-commit" "commit-msg" "pre-push")

for HOOK in "${HOOKS[@]}"; do
    TARGET="$GIT_HOOKS_DIR/$HOOK"

    if [[ -f "$TARGET" ]]; then
        rm "$TARGET"
        echo -e "${GREEN}✓ Removed $HOOK${NC}"
    else
        echo -e "${YELLOW}⚠ $HOOK not installed${NC}"
    fi
done

echo ""
echo -e "${GREEN}Git hooks uninstalled.${NC}"
echo ""
echo "To reinstall hooks, run:"
echo "  ./.githooks/install.sh"

