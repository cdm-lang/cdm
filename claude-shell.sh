#!/bin/bash
# Helper script to open a shell in a running Claude Code container

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Function to list all running Claude containers
list_sessions() {
    echo -e "${BLUE}Active Claude Code sessions:${NC}"
    docker ps --filter "name=cdm-claude-" --format "table {{.Names}}\t{{.Status}}\t{{.RunningFor}}"
}

# If session ID provided, use it
if [ -n "$1" ]; then
    SESSION_ID="$1"
    CONTAINER_NAME="cdm-claude-${SESSION_ID}"

    # Check if this specific container is running
    if ! docker ps | grep -q "$CONTAINER_NAME"; then
        echo -e "${RED}Error: Container $CONTAINER_NAME is not running${NC}"
        echo ""
        list_sessions
        exit 1
    fi

    echo -e "${GREEN}Opening shell in container: ${CONTAINER_NAME}${NC}"
    docker exec -it "$CONTAINER_NAME" /bin/bash
    exit 0
fi

# No session ID provided - check how many containers are running
CONTAINER_COUNT=$(docker ps --filter "name=cdm-claude-" --format "{{.Names}}" | wc -l)

if [ "$CONTAINER_COUNT" -eq 0 ]; then
    echo -e "${RED}Error: No Claude Code containers are running${NC}"
    echo -e "${BLUE}Start one first with: ./claude-docker.sh${NC}"
    exit 1
elif [ "$CONTAINER_COUNT" -eq 1 ]; then
    # Only one container, use it
    CONTAINER_NAME=$(docker ps --filter "name=cdm-claude-" --format "{{.Names}}")
    echo -e "${GREEN}Opening shell in container: ${CONTAINER_NAME}${NC}"
    docker exec -it "$CONTAINER_NAME" /bin/bash
else
    # Multiple containers - show list and ask user to specify
    echo -e "${YELLOW}Multiple Claude Code sessions are running:${NC}"
    echo ""
    list_sessions
    echo ""
    echo -e "${BLUE}Usage: ./claude-shell.sh <session-id>${NC}"
    echo -e "${BLUE}Example: ./claude-shell.sh 20251231-120000${NC}"
    exit 1
fi
