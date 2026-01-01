#!/bin/bash
# Helper script to run Codex CLI in Docker

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Parse arguments
BUILD_FLAG=false
CUSTOM_NAME=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --build)
            BUILD_FLAG=true
            shift
            ;;
        *)
            if [ -z "$CUSTOM_NAME" ]; then
                CUSTOM_NAME="$1"
            else
                CUSTOM_NAME="${CUSTOM_NAME}-$1"
            fi
            shift
            ;;
    esac
done

# Generate session ID and names
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

if [ -n "$CUSTOM_NAME" ]; then
    # Sanitize custom name (replace spaces and special chars with hyphens)
    SANITIZED_NAME=$(echo "$CUSTOM_NAME" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g' | sed 's/--*/-/g' | sed 's/^-//' | sed 's/-$//')
    SESSION_ID="${SANITIZED_NAME}-${TIMESTAMP}"
    BRANCH_NAME="codex-${SANITIZED_NAME}"
    WORKTREE_NAME="codex-${SANITIZED_NAME}"
else
    SESSION_ID="${TIMESTAMP}"
    BRANCH_NAME="codex-${TIMESTAMP}"
    WORKTREE_NAME="codex-${TIMESTAMP}"
fi

export CONTAINER_NAME="cdm-codex-${SESSION_ID}"
export BRANCH_NAME
export WORKTREE_NAME

echo -e "${BLUE}Starting Codex CLI in Docker...${NC}"
echo -e "${YELLOW}Session ID: ${SESSION_ID}${NC}"
echo -e "${YELLOW}Container: ${CONTAINER_NAME}${NC}"
echo -e "${YELLOW}Branch: ${BRANCH_NAME}${NC}"
echo -e "${YELLOW}Worktree: worktrees/${WORKTREE_NAME}${NC}"

# Create .env file if it doesn't exist (optional, only needed for API key auth)
if [ ! -f .env ]; then
    touch .env
fi

# Create worktrees directory if it doesn't exist
mkdir -p worktrees

# Check if worktree already exists
if [ -d "worktrees/${WORKTREE_NAME}" ]; then
    echo -e "${YELLOW}Worktree 'worktrees/${WORKTREE_NAME}' already exists${NC}"

    # Verify it's a valid git worktree
    if git worktree list | grep -q "worktrees/${WORKTREE_NAME}"; then
        echo -e "${GREEN}Reusing existing worktree: worktrees/${WORKTREE_NAME}${NC}"
        export WORKING_DIR="/workspace/worktrees/${WORKTREE_NAME}"
    else
        echo -e "${RED}Error: Directory exists but is not a valid git worktree${NC}"
        echo -e "${BLUE}Please remove the directory manually: rm -rf worktrees/${WORKTREE_NAME}${NC}"
        exit 1
    fi
else
    # Create the git worktree
    echo -e "${BLUE}Creating git worktree...${NC}"
    if git worktree add "worktrees/${WORKTREE_NAME}" -b "${BRANCH_NAME}" 2>/dev/null; then
        echo -e "${GREEN}Created worktree: worktrees/${WORKTREE_NAME}${NC}"
        export WORKING_DIR="/workspace/worktrees/${WORKTREE_NAME}"
    else
        # Branch might already exist, try without creating a new branch
        echo -e "${YELLOW}Branch '${BRANCH_NAME}' already exists, checking it out...${NC}"
        if git worktree add "worktrees/${WORKTREE_NAME}" "${BRANCH_NAME}" 2>/dev/null; then
            echo -e "${GREEN}Created worktree from existing branch: worktrees/${WORKTREE_NAME}${NC}"
            export WORKING_DIR="/workspace/worktrees/${WORKTREE_NAME}"
        else
            echo -e "${RED}Failed to create worktree${NC}"
            exit 1
        fi
    fi
fi

# Create a session file to track this container
mkdir -p .codex/sessions
cat > ".codex/sessions/${SESSION_ID}" <<EOF
CONTAINER_NAME=${CONTAINER_NAME}
SESSION_ID=${SESSION_ID}
BRANCH_NAME=${BRANCH_NAME}
WORKTREE_NAME=${WORKTREE_NAME}
STARTED=$(date)
EOF

# Build the image if it doesn't exist or if --build is passed
if [ "$BUILD_FLAG" = true ] || ! docker images | grep -q "cdm-codex"; then
    echo -e "${BLUE}Building Docker image...${NC}"
    docker-compose -f docker-compose.codex.yml build
fi

# Run the container
echo -e "${GREEN}Launching Codex CLI...${NC}"
echo -e "${BLUE}To open a shell in this session, run: ./codex-shell.sh ${SESSION_ID}${NC}"
docker-compose -f docker-compose.codex.yml run --rm --name "${CONTAINER_NAME}" codex

# Cleanup session file when done
rm -f ".codex/sessions/${SESSION_ID}"
echo -e "${YELLOW}Session ${SESSION_ID} ended${NC}"
