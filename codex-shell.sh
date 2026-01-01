#!/bin/bash
# Helper script to open a shell in a running Codex container

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Function to list all running Codex containers
list_sessions() {
    echo -e "${BLUE}Active Codex sessions:${NC}"
    docker ps --filter "name=cdm-codex-" --format "table {{.Names}}\t{{.Status}}\t{{.RunningFor}}"
}

# If session ID provided, use it
if [ -n "$1" ]; then
    # Build session ID from arguments (same logic as codex-docker.sh)
    CUSTOM_NAME=""
    for arg in "$@"; do
        if [ -z "$CUSTOM_NAME" ]; then
            CUSTOM_NAME="$arg"
        else
            CUSTOM_NAME="${CUSTOM_NAME}-$arg"
        fi
    done

    # Sanitize the name (same as codex-docker.sh)
    SANITIZED_NAME=$(echo "$CUSTOM_NAME" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g' | sed 's/--*/-/g' | sed 's/^-//' | sed 's/-$//')

    # Try to find container with this prefix
    MATCHING_CONTAINERS=$(docker ps --filter "name=cdm-codex-${SANITIZED_NAME}-" --format "{{.Names}}")

    if [ -z "$MATCHING_CONTAINERS" ]; then
        echo -e "${RED}Error: No container found matching 'cdm-codex-${SANITIZED_NAME}-*'${NC}"
        echo ""
        list_sessions
        exit 1
    fi

    # Count how many matches
    MATCH_COUNT=$(echo "$MATCHING_CONTAINERS" | wc -l | tr -d ' ')

    if [ "$MATCH_COUNT" -eq 1 ]; then
        CONTAINER_NAME="$MATCHING_CONTAINERS"
        echo -e "${GREEN}Opening shell in container: ${CONTAINER_NAME}${NC}"
        docker exec -it "$CONTAINER_NAME" /bin/bash
        exit 0
    else
        echo -e "${YELLOW}Multiple containers found matching 'cdm-codex-${SANITIZED_NAME}-*':${NC}"
        echo ""
        echo "$MATCHING_CONTAINERS" | while read -r name; do
            docker ps --filter "name=$name" --format "table {{.Names}}\t{{.Status}}\t{{.RunningFor}}"
        done
        echo ""
        echo -e "${BLUE}Please specify the full session ID (the timestamp part)${NC}"
        exit 1
    fi
fi

# No session ID provided - check how many containers are running
CONTAINER_COUNT=$(docker ps --filter "name=cdm-codex-" --format "{{.Names}}" | wc -l)

if [ "$CONTAINER_COUNT" -eq 0 ]; then
    echo -e "${RED}Error: No Codex containers are running${NC}"
    echo -e "${BLUE}Start one first with: ./codex-docker.sh${NC}"
    exit 1
elif [ "$CONTAINER_COUNT" -eq 1 ]; then
    # Only one container, use it
    CONTAINER_NAME=$(docker ps --filter "name=cdm-codex-" --format "{{.Names}}")
    echo -e "${GREEN}Opening shell in container: ${CONTAINER_NAME}${NC}"
    docker exec -it "$CONTAINER_NAME" /bin/bash
else
    # Multiple containers - show list and ask user to specify
    echo -e "${YELLOW}Multiple Codex sessions are running:${NC}"
    echo ""
    list_sessions
    echo ""
    echo -e "${BLUE}Usage: ./codex-shell.sh <session-id>${NC}"
    echo -e "${BLUE}Example: ./codex-shell.sh 20251231-120000${NC}"
    exit 1
fi
