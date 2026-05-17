//! File system scanning and directory traversal utilities.
//!
//! This module is responsible for discovering relevant source code files within a
//! target directory. It utilizes `walkdir` for efficient recursive traversal while
//! actively filtering out heavy, irrelevant directories (like `node_modules` or `target`)
//! to ensure optimal performance during the AI onboarding process.

use crate::errors::OnboarderError;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scans a target directory recursively and returns a list of valid source files.
///
/// This function traverses the directory tree starting from the provided `path`.
/// It applies strict inclusion and exclusion rules to prevent processing binaries,
/// build artifacts, or dependency folders, which saves both time and AI tokens.
///
/// Filtering Logic
/// - Directory Exclusion: Actively skips common dependency and build directories
///   like `.git`, `node_modules`, `target`, `dist`, and `build` (case-insensitive).
/// - Extension Inclusion: Only accepts files matching a hardcoded whitelist of
///   known source code and configuration extensions (e.g., `.rs`, `.py`, `.js`, `.toml`).
/// - Symlinks: Currently configured to *not* follow symbolic links to prevent
///   infinite loops or escaping the intended bounds of the target workspace.
pub fn scan_directory(path: &Path) -> Result<Vec<PathBuf>, OnboarderError> {
    // Directories to ignore (case-insensitive)
    const IGNORED_DIRS: &[&str] = &[".git", "node_modules", "target", "dist", "build"];

    // Valid file extensions to include
    const VALID_EXTENSIONS: &[&str] = &[
        "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h", "hpp", "cs", "rb",
        "php", "swift", "kt", "scala", "r", "m", "mm", "sh", "bash", "zsh", "fish", "ps1", "bat",
        "cmd", "md", "txt", "yaml", "yml", "toml", "json", "xml", "html", "css", "scss", "sass",
        "less", "sql", "graphql", "proto", "thrift", "vue", "svelte", "dart", "lua", "perl", "pl",
        "ex", "exs", "erl", "hrl", "clj", "cljs", "cljc", "elm", "hs", "ml", "mli", "fs", "fsx",
        "fsi", "v", "vhdl", "vhd", "sv", "svh", "zig", "nim",
    ];

    let mut valid_files = Vec::new();

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Filter out ignored directories
            if e.file_type().is_dir() {
                let dir_name = e.file_name().to_string_lossy().to_lowercase();
                !IGNORED_DIRS.contains(&dir_name.as_str())
            } else {
                true
            }
        })
    {
        let entry = entry.map_err(|e| {
            OnboarderError::ParsingError(format!("Failed to read directory entry: {}", e))
        })?;

        // Only process files, not directories
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Check if file has a valid extension
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            if VALID_EXTENSIONS.contains(&ext.as_str()) {
                valid_files.push(path.to_path_buf());
            }
        }
    }

    Ok(valid_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_directory_filters_ignored_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create some valid files
        fs::write(base_path.join("main.rs"), "fn main() {}").unwrap();
        fs::write(base_path.join("README.md"), "# Test").unwrap();

        // Create ignored directories with files
        fs::create_dir(base_path.join("node_modules")).unwrap();
        fs::write(base_path.join("node_modules/package.js"), "test").unwrap();

        fs::create_dir(base_path.join("target")).unwrap();
        fs::write(base_path.join("target/debug.rs"), "test").unwrap();

        let files = scan_directory(base_path).unwrap();

        // Should only find the two valid files, not the ones in ignored dirs
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("main.rs")));
        assert!(files.iter().any(|p| p.ends_with("README.md")));
        assert!(
            !files
                .iter()
                .any(|p| p.to_string_lossy().contains("node_modules"))
        );
        assert!(!files.iter().any(|p| p.to_string_lossy().contains("target")));
    }
}
