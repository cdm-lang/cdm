#!/bin/bash
# Helper script to clean up old Claude Code sessions and worktrees

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}AI Coding Assistant Cleanup Utility${NC}"
echo ""

# Show running Claude containers
echo -e "${YELLOW}Running Claude Code containers:${NC}"
RUNNING_CLAUDE=$(docker ps --filter "name=cdm-claude-" --format "{{.Names}}" 2>/dev/null | wc -l)
if [ "$RUNNING_CLAUDE" -gt 0 ]; then
    docker ps --filter "name=cdm-claude-" --format "table {{.Names}}\t{{.Status}}\t{{.RunningFor}}"
else
    echo "None"
fi
echo ""

# Show running Codex containers
echo -e "${YELLOW}Running Codex containers:${NC}"
RUNNING_CODEX=$(docker ps --filter "name=cdm-codex-" --format "{{.Names}}" 2>/dev/null | wc -l)
if [ "$RUNNING_CODEX" -gt 0 ]; then
    docker ps --filter "name=cdm-codex-" --format "table {{.Names}}\t{{.Status}}\t{{.RunningFor}}"
else
    echo "None"
fi
echo ""

# Show stopped Claude containers
echo -e "${YELLOW}Stopped Claude Code containers:${NC}"
STOPPED_CLAUDE=$(docker ps -a --filter "name=cdm-claude-" --filter "status=exited" --format "{{.Names}}" 2>/dev/null | wc -l)
if [ "$STOPPED_CLAUDE" -gt 0 ]; then
    docker ps -a --filter "name=cdm-claude-" --filter "status=exited" --format "table {{.Names}}\t{{.Status}}"
else
    echo "None"
fi
echo ""

# Show stopped Codex containers
echo -e "${YELLOW}Stopped Codex containers:${NC}"
STOPPED_CODEX=$(docker ps -a --filter "name=cdm-codex-" --filter "status=exited" --format "{{.Names}}" 2>/dev/null | wc -l)
if [ "$STOPPED_CODEX" -gt 0 ]; then
    docker ps -a --filter "name=cdm-codex-" --filter "status=exited" --format "table {{.Names}}\t{{.Status}}"
else
    echo "None"
fi
echo ""

# Function to check if a worktree branch is fully merged to main
check_worktree_merged() {
    local worktree_path="$1"
    local worktree_name="$2"

    # Get the branch name for this worktree
    local branch_info=$(git worktree list --porcelain | grep -A2 "worktree.*$worktree_path$" | grep "branch" | sed 's/branch refs\/heads\///')

    if [ -z "$branch_info" ]; then
        # Try to get branch from the worktree itself
        branch_info=$(cd "$worktree_path" && git rev-parse --abbrev-ref HEAD 2>/dev/null)
    fi

    if [ -z "$branch_info" ] || [ "$branch_info" = "HEAD" ]; then
        echo "unknown"
        return
    fi

    # Check if there are any commits in the branch that are not in main
    local unmerged_commits=$(git log main.."$branch_info" --oneline 2>/dev/null | wc -l)

    if [ "$unmerged_commits" -eq 0 ]; then
        echo "merged"
    else
        echo "unmerged:$unmerged_commits"
    fi
}

# Show worktrees with merge status
echo -e "${YELLOW}Git worktrees:${NC}"
MERGED_WORKTREES=()
UNMERGED_WORKTREES=()

if [ -d "worktrees" ] && [ "$(ls -A worktrees 2>/dev/null)" ]; then
    for worktree in worktrees/*/; do
        if [ -d "$worktree" ]; then
            worktree_name=$(basename "$worktree")
            worktree_path=$(realpath "$worktree")

            # Check if it's a valid git worktree
            if git worktree list | grep -q "$worktree_path"; then
                # Get branch name
                branch_info=$(cd "$worktree_path" && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")

                # Check merge status
                merge_status=$(check_worktree_merged "$worktree_path" "$worktree_name")

                if [ "$merge_status" = "merged" ]; then
                    echo -e "  ${GREEN}✓${NC} $worktree_name ${CYAN}[$branch_info]${NC} - ${GREEN}fully merged to main${NC}"
                    MERGED_WORKTREES+=("$worktree_name")
                elif [[ "$merge_status" == unmerged:* ]]; then
                    commit_count="${merge_status#unmerged:}"
                    echo -e "  ${YELLOW}!${NC} $worktree_name ${CYAN}[$branch_info]${NC} - ${YELLOW}$commit_count commit(s) not in main${NC}"
                    UNMERGED_WORKTREES+=("$worktree_name")
                else
                    echo -e "  ${RED}?${NC} $worktree_name ${CYAN}[$branch_info]${NC} - ${RED}unknown status${NC}"
                    UNMERGED_WORKTREES+=("$worktree_name")
                fi
            else
                echo -e "  ${RED}✗${NC} $worktree_name - ${RED}orphaned (not a valid worktree)${NC}"
            fi
        fi
    done
else
    echo "None"
fi
echo ""

# Summary
MERGED_COUNT=${#MERGED_WORKTREES[@]}
UNMERGED_COUNT=${#UNMERGED_WORKTREES[@]}
echo -e "${BLUE}Summary:${NC} $MERGED_COUNT merged worktree(s), $UNMERGED_COUNT with unmerged commits"
echo ""

# Cleanup options
echo -e "${BLUE}What would you like to clean up?${NC}"
echo "1. Remove stopped containers only"
echo "2. Remove orphaned worktrees only"
echo "3. Remove merged worktrees + their containers (safe cleanup)"
echo "4. Remove all stopped containers + orphaned worktrees"
echo "5. Cancel"
echo ""
read -p "Enter your choice (1-5): " choice

case $choice in
    1)
        TOTAL_STOPPED=$((STOPPED_CLAUDE + STOPPED_CODEX))
        if [ "$TOTAL_STOPPED" -gt 0 ]; then
            echo -e "${GREEN}Removing stopped containers...${NC}"
            docker ps -a --filter "name=cdm-claude-" --filter "status=exited" --format "{{.Names}}" | xargs -r docker rm
            docker ps -a --filter "name=cdm-codex-" --filter "status=exited" --format "{{.Names}}" | xargs -r docker rm
            echo -e "${GREEN}Done!${NC}"
        else
            echo -e "${YELLOW}No stopped containers to remove${NC}"
        fi
        ;;
    2)
        echo -e "${GREEN}Removing orphaned worktrees...${NC}"
        if [ -d "worktrees" ]; then
            ls -1 worktrees/ 2>/dev/null | while read -r worktree; do
                worktree_path=$(realpath "worktrees/$worktree")
                if ! git worktree list | grep -q "$worktree_path"; then
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
        # Remove merged worktrees and their containers
        if [ "$MERGED_COUNT" -eq 0 ]; then
            echo -e "${YELLOW}No merged worktrees to remove${NC}"
            exit 0
        fi

        echo -e "${GREEN}Removing $MERGED_COUNT merged worktree(s) and their containers...${NC}"
        for worktree_name in "${MERGED_WORKTREES[@]}"; do
            worktree_path=$(realpath "worktrees/$worktree_name")

            # Get the branch name before removing
            branch_name=$(cd "$worktree_path" && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

            echo -e "  Removing worktree: ${CYAN}$worktree_name${NC}"

            # Remove associated docker containers (both running and stopped)
            # Container names typically include the worktree name
            claude_containers=$(docker ps -a --filter "name=cdm-claude-.*$worktree_name" --format "{{.Names}}" 2>/dev/null || true)
            codex_containers=$(docker ps -a --filter "name=cdm-codex-.*$worktree_name" --format "{{.Names}}" 2>/dev/null || true)

            if [ -n "$claude_containers" ]; then
                echo "    Removing Claude container(s): $claude_containers"
                echo "$claude_containers" | xargs -r docker rm -f 2>/dev/null || true
            fi
            if [ -n "$codex_containers" ]; then
                echo "    Removing Codex container(s): $codex_containers"
                echo "$codex_containers" | xargs -r docker rm -f 2>/dev/null || true
            fi

            # Remove the git worktree properly
            git worktree remove "$worktree_path" --force 2>/dev/null || {
                echo "    Warning: Could not remove worktree via git, removing directory manually"
                rm -rf "$worktree_path"
            }

            # Optionally delete the branch if it's fully merged
            if [ -n "$branch_name" ] && [ "$branch_name" != "main" ] && [ "$branch_name" != "master" ]; then
                read -p "    Delete branch '$branch_name'? (y/N): " delete_branch
                if [ "$delete_branch" = "y" ] || [ "$delete_branch" = "Y" ]; then
                    git branch -d "$branch_name" 2>/dev/null && echo "    Deleted branch: $branch_name" || echo "    Could not delete branch: $branch_name"
                fi
            fi
        done

        # Prune worktree references
        git worktree prune 2>/dev/null || true

        echo -e "${GREEN}Done!${NC}"
        ;;
    4)
        # Remove stopped containers
        TOTAL_STOPPED=$((STOPPED_CLAUDE + STOPPED_CODEX))
        if [ "$TOTAL_STOPPED" -gt 0 ]; then
            echo -e "${GREEN}Removing stopped containers...${NC}"
            docker ps -a --filter "name=cdm-claude-" --filter "status=exited" --format "{{.Names}}" | xargs -r docker rm 2>/dev/null || true
            docker ps -a --filter "name=cdm-codex-" --filter "status=exited" --format "{{.Names}}" | xargs -r docker rm 2>/dev/null || true
        fi

        # Remove orphaned worktrees
        echo -e "${GREEN}Removing orphaned worktrees...${NC}"
        if [ -d "worktrees" ]; then
            ls -1 worktrees/ 2>/dev/null | while read -r worktree; do
                worktree_path=$(realpath "worktrees/$worktree")
                if ! git worktree list | grep -q "$worktree_path"; then
                    echo "  Removing worktrees/$worktree"
                    rm -rf "worktrees/$worktree"
                fi
            done
        fi

        # Clean up session files
        if [ -d ".claude/sessions" ]; then
            echo -e "${GREEN}Cleaning up Claude session files...${NC}"
            rm -rf .claude/sessions/*
        fi
        if [ -d ".codex/sessions" ]; then
            echo -e "${GREEN}Cleaning up Codex session files...${NC}"
            rm -rf .codex/sessions/*
        fi

        echo -e "${GREEN}All cleanup complete!${NC}"
        ;;
    5)
        echo -e "${YELLOW}Cancelled${NC}"
        exit 0
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac
