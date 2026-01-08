#!/bin/bash
# Helper script to clean up old Claude Code sessions and worktrees

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BLUE}AI Coding Assistant Cleanup Utility${NC}"
echo ""

# Function to check if a worktree branch is fully merged to main
check_worktree_merged() {
    local worktree_path="$1"

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
        echo "unmerged"
    fi
}

# Function to extract session name from container name
# Container names are like: cdm-claude-<session> or cdm-codex-<session>
extract_session_name() {
    local container_name="$1"
    echo "$container_name" | sed -E 's/^cdm-(claude|codex)-//'
}

# Helper functions to simulate associative arrays using parallel indexed arrays
# Session data stored in parallel arrays indexed by position
SESSION_NAMES=()
SESSION_HAS_WORKTREE=()
SESSION_HAS_CONTAINER=()
SESSION_CONTAINER_NAMES=()
SESSION_MERGE_STATUS=()

# Find session index by name, sets FOUND_IDX to index or -1 if not found
find_session_index() {
    local name="$1"
    local i
    FOUND_IDX=-1
    for i in "${!SESSION_NAMES[@]}"; do
        if [ "${SESSION_NAMES[$i]}" = "$name" ]; then
            FOUND_IDX=$i
            return
        fi
    done
}

# Add or update session data, sets RESULT_IDX to the index
add_session() {
    local name="$1"
    find_session_index "$name"
    if [ "$FOUND_IDX" = "-1" ]; then
        SESSION_NAMES+=("$name")
        SESSION_HAS_WORKTREE+=("no")
        SESSION_HAS_CONTAINER+=("no")
        SESSION_CONTAINER_NAMES+=("")
        SESSION_MERGE_STATUS+=("n/a")
        RESULT_IDX=$((${#SESSION_NAMES[@]} - 1))
    else
        RESULT_IDX=$FOUND_IDX
    fi
}

# Collect worktrees
if [ -d "worktrees" ] && [ "$(ls -A worktrees 2>/dev/null)" ]; then
    for worktree in worktrees/*/; do
        if [ -d "$worktree" ]; then
            worktree_name=$(basename "$worktree")
            worktree_path=$(realpath "$worktree")

            add_session "$worktree_name"
            idx=$RESULT_IDX
            SESSION_HAS_WORKTREE[$idx]="yes"

            # Check if it's a valid git worktree and get merge status
            if git worktree list | grep -q "$worktree_path"; then
                merge_status=$(check_worktree_merged "$worktree_path")
                SESSION_MERGE_STATUS[$idx]="$merge_status"
            else
                SESSION_MERGE_STATUS[$idx]="orphaned"
            fi
        fi
    done
fi

# Collect containers (both Claude and Codex)
for container in $(docker ps -a --filter "name=cdm-claude-" --format "{{.Names}}" 2>/dev/null); do
    session_name=$(extract_session_name "$container")
    add_session "$session_name"
    idx=$RESULT_IDX
    SESSION_HAS_CONTAINER[$idx]="yes"
    if [ -n "${SESSION_CONTAINER_NAMES[$idx]}" ]; then
        SESSION_CONTAINER_NAMES[$idx]="${SESSION_CONTAINER_NAMES[$idx]} $container"
    else
        SESSION_CONTAINER_NAMES[$idx]="$container"
    fi
done

for container in $(docker ps -a --filter "name=cdm-codex-" --format "{{.Names}}" 2>/dev/null); do
    session_name=$(extract_session_name "$container")
    add_session "$session_name"
    idx=$RESULT_IDX
    SESSION_HAS_CONTAINER[$idx]="yes"
    if [ -n "${SESSION_CONTAINER_NAMES[$idx]}" ]; then
        SESSION_CONTAINER_NAMES[$idx]="${SESSION_CONTAINER_NAMES[$idx]} $container"
    else
        SESSION_CONTAINER_NAMES[$idx]="$container"
    fi
done

# Display unified table
echo -e "${YELLOW}Sessions:${NC}"
echo ""

if [ ${#SESSION_NAMES[@]} -eq 0 ]; then
    echo "No worktrees or containers found."
    echo ""
    exit 0
fi

# Print table header
printf "  ${BOLD}%-3s %-40s %-10s %-12s %-10s${NC}\n" "#" "NAME" "WORKTREE" "CONTAINER" "UNMERGED"
printf "  %-3s %-40s %-10s %-12s %-10s\n" "---" "----------------------------------------" "----------" "------------" "----------"

# Sort session names and create display order
SORTED_INDICES=()
for i in "${!SESSION_NAMES[@]}"; do
    SORTED_INDICES+=("$i")
done

# Simple bubble sort by name
for ((i=0; i<${#SORTED_INDICES[@]}; i++)); do
    for ((j=i+1; j<${#SORTED_INDICES[@]}; j++)); do
        idx_i=${SORTED_INDICES[$i]}
        idx_j=${SORTED_INDICES[$j]}
        if [[ "${SESSION_NAMES[$idx_i]}" > "${SESSION_NAMES[$idx_j]}" ]]; then
            SORTED_INDICES[$i]=$idx_j
            SORTED_INDICES[$j]=$idx_i
        fi
    done
done

# Track merged sessions for option 2
MERGED_INDICES=()
display_num=1

for idx in "${SORTED_INDICES[@]}"; do
    session="${SESSION_NAMES[$idx]}"
    has_worktree="${SESSION_HAS_WORKTREE[$idx]}"
    has_container="${SESSION_HAS_CONTAINER[$idx]}"
    merge_status="${SESSION_MERGE_STATUS[$idx]}"

    # Determine unmerged display (pad first, then colorize)
    if [ "$has_worktree" = "no" ]; then
        unmerged_display="n/a"
    elif [ "$merge_status" = "merged" ]; then
        unmerged_display="${GREEN}no${NC}"
        MERGED_INDICES+=("$idx")
    elif [ "$merge_status" = "unmerged" ]; then
        unmerged_display="${YELLOW}yes${NC}"
    elif [ "$merge_status" = "orphaned" ]; then
        unmerged_display="${RED}orphaned${NC}"
    else
        unmerged_display="${RED}unknown${NC}"
    fi

    # Color the worktree/container columns (use fixed-width padding before colors)
    if [ "$has_worktree" = "yes" ]; then
        worktree_display="${GREEN}yes${NC}       "
    else
        worktree_display="no        "
    fi

    if [ "$has_container" = "yes" ]; then
        container_display="${GREEN}yes${NC}         "
    else
        container_display="no          "
    fi

    printf "  %-3s %-40s %b %b %b\n" "$display_num" "$session" "$worktree_display" "$container_display" "$unmerged_display"
    ((display_num++))
done

echo ""

# Summary
MERGED_COUNT=${#MERGED_INDICES[@]}
TOTAL_COUNT=${#SESSION_NAMES[@]}
echo -e "${BLUE}Summary:${NC} $TOTAL_COUNT session(s), $MERGED_COUNT fully merged"
echo ""

# Cleanup options
echo -e "${BLUE}What would you like to clean up?${NC}"
echo "1. Choose which sessions to remove (multiselect)"
echo "2. Remove merged worktrees + their containers (safe cleanup)"
echo "3. Cancel"
echo ""
read -p "Enter your choice (1-3): " choice

# Function to remove a session by index
remove_session_by_index() {
    local idx="$1"
    local session_name="${SESSION_NAMES[$idx]}"
    local has_worktree="${SESSION_HAS_WORKTREE[$idx]}"
    local has_container="${SESSION_HAS_CONTAINER[$idx]}"

    echo -e "  Removing session: ${CYAN}$session_name${NC}"

    # Remove containers
    if [ "$has_container" = "yes" ]; then
        containers="${SESSION_CONTAINER_NAMES[$idx]}"
        for container in $containers; do
            echo "    Removing container: $container"
            docker rm -f "$container" 2>/dev/null || true
        done
    fi

    # Remove worktree
    if [ "$has_worktree" = "yes" ]; then
        worktree_path=$(realpath "worktrees/$session_name" 2>/dev/null || echo "")
        if [ -n "$worktree_path" ] && [ -d "$worktree_path" ]; then
            # Get branch name before removing
            branch_name=$(cd "$worktree_path" && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

            echo "    Removing worktree: $session_name"
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
        fi
    fi
}

case $choice in
    1)
        # Multiselect UI
        echo ""
        echo -e "${BLUE}Enter the numbers of sessions to remove (comma or space separated):${NC}"
        echo -e "${YELLOW}Example: 1,3,5 or 1 3 5${NC}"
        echo ""
        read -p "Sessions to remove: " selections

        if [ -z "$selections" ]; then
            echo -e "${YELLOW}No sessions selected${NC}"
            exit 0
        fi

        # Parse selections (handle both comma and space separated)
        selections=$(echo "$selections" | tr ',' ' ')
        SELECTED_INDICES=()

        for sel in $selections; do
            # Validate it's a number
            if [[ "$sel" =~ ^[0-9]+$ ]]; then
                display_idx=$((sel - 1))
                if [ $display_idx -ge 0 ] && [ $display_idx -lt ${#SORTED_INDICES[@]} ]; then
                    # Map display number to actual session index
                    actual_idx=${SORTED_INDICES[$display_idx]}
                    SELECTED_INDICES+=("$actual_idx")
                else
                    echo -e "${RED}Invalid selection: $sel (out of range)${NC}"
                fi
            else
                echo -e "${RED}Invalid selection: $sel (not a number)${NC}"
            fi
        done

        if [ ${#SELECTED_INDICES[@]} -eq 0 ]; then
            echo -e "${YELLOW}No valid sessions selected${NC}"
            exit 0
        fi

        echo ""
        echo -e "${YELLOW}The following sessions will be removed:${NC}"
        for idx in "${SELECTED_INDICES[@]}"; do
            echo "  - ${SESSION_NAMES[$idx]}"
        done
        echo ""
        read -p "Proceed? (y/N): " confirm

        if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
            echo -e "${YELLOW}Cancelled${NC}"
            exit 0
        fi

        echo ""
        echo -e "${GREEN}Removing selected sessions...${NC}"
        for idx in "${SELECTED_INDICES[@]}"; do
            remove_session_by_index "$idx"
        done

        # Prune worktree references
        git worktree prune 2>/dev/null || true

        echo -e "${GREEN}Done!${NC}"
        ;;
    2)
        # Remove merged worktrees and their containers
        if [ "$MERGED_COUNT" -eq 0 ]; then
            echo -e "${YELLOW}No merged worktrees to remove${NC}"
            exit 0
        fi

        echo -e "${GREEN}Removing $MERGED_COUNT merged session(s)...${NC}"
        for idx in "${MERGED_INDICES[@]}"; do
            remove_session_by_index "$idx"
        done

        # Prune worktree references
        git worktree prune 2>/dev/null || true

        echo -e "${GREEN}Done!${NC}"
        ;;
    3)
        echo -e "${YELLOW}Cancelled${NC}"
        exit 0
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac
