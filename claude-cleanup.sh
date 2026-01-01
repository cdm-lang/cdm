#!/bin/bash
# Helper script to clean up old Claude Code sessions and worktrees

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}Claude Code Cleanup Utility${NC}"
echo ""

# Show running containers
echo -e "${YELLOW}Running containers:${NC}"
RUNNING=$(docker ps --filter "name=cdm-claude-" --format "{{.Names}}" | wc -l)
if [ "$RUNNING" -gt 0 ]; then
    docker ps --filter "name=cdm-claude-" --format "table {{.Names}}\t{{.Status}}\t{{.RunningFor}}"
else
    echo "None"
fi
echo ""

# Show stopped containers
echo -e "${YELLOW}Stopped containers:${NC}"
STOPPED=$(docker ps -a --filter "name=cdm-claude-" --filter "status=exited" --format "{{.Names}}" | wc -l)
if [ "$STOPPED" -gt 0 ]; then
    docker ps -a --filter "name=cdm-claude-" --filter "status=exited" --format "table {{.Names}}\t{{.Status}}"
else
    echo "None"
fi
echo ""

# Show worktrees
echo -e "${YELLOW}Git worktrees:${NC}"
if [ -d "worktrees" ] && [ "$(ls -A worktrees 2>/dev/null)" ]; then
    ls -1 worktrees/ | while read -r worktree; do
        BRANCH_EXISTS=$(git worktree list | grep -c "worktrees/$worktree" || true)
        if [ "$BRANCH_EXISTS" -gt 0 ]; then
            echo "  ✓ $worktree (active)"
        else
            echo "  ✗ $worktree (orphaned)"
        fi
    done
else
    echo "None"
fi
echo ""

# Cleanup options
echo -e "${BLUE}What would you like to clean up?${NC}"
echo "1. Remove stopped containers only"
echo "2. Remove orphaned worktrees only"
echo "3. Remove all (stopped containers + orphaned worktrees)"
echo "4. Cancel"
echo ""
read -p "Enter your choice (1-4): " choice

case $choice in
    1)
        if [ "$STOPPED" -gt 0 ]; then
            echo -e "${GREEN}Removing stopped containers...${NC}"
            docker ps -a --filter "name=cdm-claude-" --filter "status=exited" --format "{{.Names}}" | xargs -r docker rm
            echo -e "${GREEN}Done!${NC}"
        else
            echo -e "${YELLOW}No stopped containers to remove${NC}"
        fi
        ;;
    2)
        echo -e "${GREEN}Removing orphaned worktrees...${NC}"
        if [ -d "worktrees" ]; then
            ls -1 worktrees/ 2>/dev/null | while read -r worktree; do
                BRANCH_EXISTS=$(git worktree list | grep -c "worktrees/$worktree" || true)
                if [ "$BRANCH_EXISTS" -eq 0 ]; then
                    echo "  Removing worktrees/$worktree"
                    rm -rf "worktrees/$worktree"
                fi
            done
            echo -e "${GREEN}Done!${NC}"
        else
            echo -e "${YELLOW}No worktrees directory found${NC}"
        fi
        ;;
    3)
        # Remove stopped containers
        if [ "$STOPPED" -gt 0 ]; then
            echo -e "${GREEN}Removing stopped containers...${NC}"
            docker ps -a --filter "name=cdm-claude-" --filter "status=exited" --format "{{.Names}}" | xargs -r docker rm
        fi

        # Remove orphaned worktrees
        echo -e "${GREEN}Removing orphaned worktrees...${NC}"
        if [ -d "worktrees" ]; then
            ls -1 worktrees/ 2>/dev/null | while read -r worktree; do
                BRANCH_EXISTS=$(git worktree list | grep -c "worktrees/$worktree" || true)
                if [ "$BRANCH_EXISTS" -eq 0 ]; then
                    echo "  Removing worktrees/$worktree"
                    rm -rf "worktrees/$worktree"
                fi
            done
        fi

        # Clean up session files
        if [ -d ".claude/sessions" ]; then
            echo -e "${GREEN}Cleaning up session files...${NC}"
            rm -rf .claude/sessions/*
        fi

        echo -e "${GREEN}All cleanup complete!${NC}"
        ;;
    4)
        echo -e "${YELLOW}Cancelled${NC}"
        exit 0
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac
