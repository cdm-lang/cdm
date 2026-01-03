#!/bin/bash
# Entrypoint script to fix git worktree paths inside the container

set -e

# Fix ownership of Rust build cache directory (only top-level, not recursive)
# This is fast and sufficient - Rust will create subdirs with correct ownership
if [ -d "/var/tmp/rust-build" ]; then
    # Only fix if not already owned by current user
    if [ "$(stat -c '%u' /var/tmp/rust-build 2>/dev/null || stat -f '%u' /var/tmp/rust-build)" != "$(id -u)" ]; then
        sudo chown "$(id -u):$(id -g)" /var/tmp/rust-build 2>/dev/null || true
    fi
else
    sudo mkdir -p /var/tmp/rust-build
    sudo chown "$(id -u):$(id -g)" /var/tmp/rust-build
fi

# Fix git worktree paths and add safe.directory for each worktree
if [ -d "/workspace/worktrees" ]; then
    for worktree in /workspace/worktrees/*/; do
        # Remove trailing slash for clean path
        worktree_path="${worktree%/}"

        # Add safe.directory for this worktree (using system config since user config is read-only)
        sudo git config --system --add safe.directory "$worktree_path" 2>/dev/null || true

        if [ -f "${worktree}.git" ]; then
            # Extract worktree name
            WORKTREE_NAME=$(basename "$worktree_path")

            # Use relative path that works on both host and container
            # From worktrees/foo/, the relative path to .git/worktrees/foo is ../../.git/worktrees/foo
            RELATIVE_GITDIR="../../.git/worktrees/${WORKTREE_NAME}"

            # Only update if not already using relative path
            CURRENT_GITDIR=$(cat "${worktree}.git" | sed 's/gitdir: //')
            if [ "$CURRENT_GITDIR" != "$RELATIVE_GITDIR" ]; then
                echo "gitdir: ${RELATIVE_GITDIR}" > "${worktree}.git"
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
