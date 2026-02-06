// Project Scanner
// Discovers all .cdm files in a project directory for dependency analysis

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Default directory names to ignore during scanning
const DEFAULT_IGNORE_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
];

/// Scans a project directory for .cdm files
pub struct ProjectScanner {
    root: PathBuf,
    ignore_dirs: HashSet<String>,
}

impl ProjectScanner {
    /// Create a new scanner rooted at the given directory
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            ignore_dirs: DEFAULT_IGNORE_DIRS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Scan the project directory and return all .cdm files
    pub fn scan(&self) -> Result<Vec<PathBuf>, io::Error> {
        let mut files = Vec::new();
        self.scan_directory(&self.root, &mut files)?;

        // Sort for deterministic ordering
        files.sort();

        Ok(files)
    }

    /// Recursively scan a directory for .cdm files
    fn scan_directory(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), io::Error> {
        // Skip if directory doesn't exist or isn't a directory
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if path.is_dir() {
                // Skip ignored directories
                if self.ignore_dirs.contains(file_name_str.as_ref()) {
                    continue;
                }
                // Skip hidden directories (starting with .)
                if file_name_str.starts_with('.') {
                    continue;
                }
                // Recurse into subdirectory
                self.scan_directory(&path, files)?;
            } else if path.is_file() {
                // Check for .cdm extension
                if let Some(ext) = path.extension() {
                    if ext == "cdm" {
                        files.push(path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Find the project root directory for a given file
    ///
    /// Looks for:
    /// 1. A .git directory (indicating git repository root)
    /// 2. Falls back to the parent directory of the file
    pub fn find_project_root(file_path: &Path) -> Option<PathBuf> {
        // Ensure we have an absolute path
        let absolute_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(file_path)
        };

        // Get the parent directory of the file
        let mut current = absolute_path.parent()?.to_path_buf();

        // Walk up the directory tree looking for .git
        loop {
            let git_dir = current.join(".git");
            if git_dir.exists() {
                return Some(current);
            }

            // Move to parent
            match current.parent() {
                Some(parent) => {
                    if parent == current {
                        // Reached root, no .git found
                        break;
                    }
                    current = parent.to_path_buf();
                }
                None => break,
            }
        }

        // Fall back to the file's parent directory
        absolute_path.parent().map(|p| p.to_path_buf())
    }
}

#[cfg(test)]
#[path = "project_scanner/project_scanner_tests.rs"]
mod project_scanner_tests;
