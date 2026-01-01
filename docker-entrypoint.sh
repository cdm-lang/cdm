#!/bin/bash
# Entrypoint script to fix git worktree paths inside the container

set -e

# Fix ownership of /workspace/target if running as non-root
if [ -d "/workspace/target" ] && [ "$(id -u)" != "0" ]; then
    sudo chown -R "$(id -u):$(id -g)" /workspace/target 2>/dev/null || true
fi

# Fix git worktree paths if we're in a worktree
if [ -d "/workspace/worktrees" ]; then
    for worktree in /workspace/worktrees/*/; do
        if [ -f "${worktree}.git" ]; then
            # Read the current gitdir path
            GITDIR=$(cat "${worktree}.git" | sed 's/gitdir: //')

            # If it contains the host path, fix it
            if echo "$GITDIR" | grep -q "/Users/"; then
                # Extract just the worktree name
                WORKTREE_NAME=$(basename "$worktree")

                # Update to container path
                echo "gitdir: /workspace/.git/worktrees/${WORKTREE_NAME}" > "${worktree}.git"

                echo "Fixed git path for worktree: ${WORKTREE_NAME}"
            fi
        fi
    done
fi

# Set up worktree-specific .claude directory
# Get the current working directory (will be set to the worktree path)
CURRENT_DIR=$(pwd)

# Remove existing /home/claude/.claude if it exists (could be from previous container)
if [ -e "/home/claude/.claude" ] && [ ! -L "/home/claude/.claude" ]; then
    rm -rf /home/claude/.claude
fi

# Create .claude directory in the worktree if it doesn't exist
if [ ! -d "${CURRENT_DIR}/.claude" ]; then
    mkdir -p "${CURRENT_DIR}/.claude"
    chown -R $(id -u):$(id -g) "${CURRENT_DIR}/.claude"
fi

# Create symlink from /home/claude/.claude to worktree-specific .claude
if [ -L "/home/claude/.claude" ]; then
    rm /home/claude/.claude
fi
ln -s "${CURRENT_DIR}/.claude" /home/claude/.claude

echo "Using worktree-specific session storage: ${CURRENT_DIR}/.claude"

# Execute the main command
exec "$@"
