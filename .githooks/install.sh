#!/bin/bash
#
# Install git hooks for git-flow enforcement
#
# Usage: ./.githooks/install.sh
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

echo -e "${BLUE}Installing git hooks for git-flow enforcement...${NC}"
echo ""

# Check if we're in a git repository
if [[ ! -d "$REPO_ROOT/.git" ]]; then
    echo -e "${RED}Error: Not a git repository.${NC}"
    echo "Run this script from the repository root."
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$GIT_HOOKS_DIR"

# List of hooks to install
HOOKS=("pre-commit" "commit-msg" "pre-push")

for HOOK in "${HOOKS[@]}"; do
    SOURCE="$SCRIPT_DIR/$HOOK"
    TARGET="$GIT_HOOKS_DIR/$HOOK"

    if [[ -f "$SOURCE" ]]; then
        # Backup existing hook if it exists and is different
        if [[ -f "$TARGET" ]]; then
            if ! cmp -s "$SOURCE" "$TARGET"; then
                BACKUP="$TARGET.backup.$(date +%Y%m%d%H%M%S)"
                mv "$TARGET" "$BACKUP"
                echo -e "${YELLOW}Backed up existing $HOOK to $BACKUP${NC}"
            fi
        fi

        # Copy the hook
        cp "$SOURCE" "$TARGET"
        chmod +x "$TARGET"
        echo -e "${GREEN}✓ Installed $HOOK${NC}"
    else
        echo -e "${YELLOW}⚠ Hook $HOOK not found in .githooks/${NC}"
    fi
done

echo ""
echo -e "${GREEN}Git hooks installed successfully!${NC}"
echo ""
echo "Hooks installed:"
echo "  • pre-commit  - Prevents commits to protected branches, runs formatters"
echo "  • commit-msg  - Validates conventional commit message format"
echo "  • pre-push    - Prevents pushes to protected branches, validates branch naming"
echo ""
echo "To uninstall hooks, run:"
echo "  rm .git/hooks/{pre-commit,commit-msg,pre-push}"
echo ""
echo "To skip hooks temporarily (not recommended), use:"
echo "  git commit --no-verify"
echo "  git push --no-verify"

