#!/bin/bash
# Entrypoint script to fix git worktree paths inside the container

set -e

# Fix ownership of /workspace/target if running as non-root
if [ -d "/workspace/target" ] && [ "$(id -u)" != "0" ]; then
    sudo chown -R "$(id -u):$(id -g)" /workspace/target 2>/dev/null || true
fi

# Fix ownership of Rust build cache directory (Docker volume may be owned by root)
if [ -d "/var/tmp/rust-build" ]; then
    sudo chown -R "$(id -u):$(id -g)" /var/tmp/rust-build 2>/dev/null || true
else
    sudo mkdir -p /var/tmp/rust-build
    sudo chown -R "$(id -u):$(id -g)" /var/tmp/rust-build
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

# The ~/.claude directory is now mounted from the host via docker-compose
# This preserves credentials and settings across container restarts
if [ -d "/home/claude/.claude" ]; then
    echo "Using host ~/.claude directory for credentials and settings"
else
    # Fallback: create local .claude directory if not mounted
    mkdir -p /home/claude/.claude
    echo "Created local ~/.claude directory"
fi

# Execute the main command
exec "$@"
