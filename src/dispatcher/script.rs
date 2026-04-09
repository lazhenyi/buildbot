//! Python script scanner — parses `/.ci/*.py` files, extracts imports,
//! validates against requirements.txt in strict dependency mode.

use std::collections::HashSet;
use std::path::Path;

/// Python import mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    /// Strict mode: only import modules listed in requirements.txt
    Strict,
    /// Allow all imports (full network access)
    AllowAll,
}

/// Scanned script metadata
#[derive(Debug, Clone)]
pub struct ScriptInfo {
    /// Full filename including extension (e.g. "01_build.py")
    pub filename: String,
    /// Sort key extracted from numeric prefix (e.g. "01" → 1)
    pub sort_key: i32,
    /// Human-readable job name from filename stem (e.g. "01_build.py" → "build")
    pub job_name: String,
    /// Absolute or relative path within the repository
    pub path: String,
    /// Modules imported by this script (only external/non-stdlib)
    pub imports: Vec<String>,
    /// Whether dependency validation passed
    pub deps_valid: bool,
    /// List of unapproved imports (only filled in strict mode)
    pub unapproved_imports: Vec<String>,
}

impl ScriptInfo {
    /// Sort key for ordering: primary = numeric prefix, secondary = alphabetical
    pub fn sort_key(&self) -> (i32, String) {
        (self.sort_key, self.filename.clone())
    }
}

/// Scans Python scripts in a directory, extracts imports, validates deps.
pub struct ScriptScanner {
    mode: ImportMode,
    /// Whitelist of allowed module names (from requirements.txt)
    allowed_modules: HashSet<String>,
}

impl ScriptScanner {
    /// Create a scanner in the given import mode.
    /// `requirements_content` is the raw contents of requirements.txt (may be empty).
    pub fn new(mode: ImportMode, requirements_content: &str) -> Self {
        let allowed_modules = if mode == ImportMode::Strict {
            Self::parse_requirements(requirements_content)
        } else {
            HashSet::new()
        };
        Self {
            mode,
            allowed_modules,
        }
    }

    /// Parse requirements.txt into a set of module names (no version specifiers).
    fn parse_requirements(content: &str) -> HashSet<String> {
        let mut modules = HashSet::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
                continue;
            }
            // Strip version specifiers: flask>=2.0 → flask, requests==2.28.0 → requests
            let name = line
                .split(|c: char| c == '<' || c == '>' || c == '=' || c == '!' || c == '[')
                .next()
                .unwrap_or(line)
                .trim()
                .split('/')
                .last()
                .unwrap_or(line)
                .trim()
                .to_lowercase()
                .to_string();
            if !name.is_empty() {
                modules.insert(name);
            }
        }
        modules
    }

    /// Scan all `*.py` files under the given directory recursively.
    /// Returns script infos sorted by (sort_key, filename).
    pub fn scan_directory(&self, dir: &Path) -> Vec<ScriptInfo> {
        let mut scripts = Vec::new();
        if !dir.is_dir() {
            return scripts;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read CI directory {:?}: {}", dir, e);
                return scripts;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scripts.extend(self.scan_directory(&path));
                continue;
            }

            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if ext != "py" {
                continue;
            }

            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if let Some(info) = self.scan_file(&path, &filename) {
                scripts.push(info);
            }
        }

        // Sort by numeric prefix first, then alphabetically
        scripts.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        scripts
    }

    /// Scan a single `.py` file, parse imports.
    fn scan_file(&self, path: &Path, filename: &str) -> Option<ScriptInfo> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read script {:?}: {}", path, e);
                return None;
            }
        };

        let imports = self.extract_imports(&content);
        let (deps_valid, unapproved) = self.validate_dependencies(&imports);
        let sort_key = Self::extract_sort_key(filename);
        let job_name = Self::extract_job_name(filename);

        Some(ScriptInfo {
            filename: filename.to_string(),
            sort_key,
            job_name,
            path: path.to_string_lossy().to_string(),
            imports,
            deps_valid,
            unapproved_imports: unapproved,
        })
    }

    /// Extract all imported module names from Python source.
    /// Handles: `import foo`, `import foo as bar`, `from foo import bar`, `from foo import *`.
    fn extract_imports(&self, source: &str) -> Vec<String> {
        let mut imports = Vec::new();
        let mut seen = HashSet::new();

        for line in source.lines() {
            let line = line.trim();

            // Skip non-import lines
            if !line.starts_with("import ") && !line.starts_with("from ") {
                continue;
            }

            // Skip multi-line imports (simple heuristic: line ends with ',')
            if line.ends_with(',') {
                continue;
            }

            if line.starts_with("import ") {
                // `import a.b.c` → extract "a"
                let body = line.trim_start_matches("import ").trim();
                let first = body.split('.').next().unwrap_or(body).split_whitespace().next().unwrap_or(body);
                let name = first.split_ascii_whitespace().next().unwrap_or(first);
                if !name.is_empty() && !seen.contains(name) {
                    seen.insert(name.to_string());
                    imports.push(name.to_string());
                }
            } else if line.starts_with("from ") {
                // `from foo import bar` → extract "foo"
                let body = line.trim_start_matches("from ").trim();
                let name = body.split_whitespace().next().unwrap_or(body)
                    .split('.').next().unwrap_or(body);
                if !name.is_empty() && !seen.contains(name) {
                    seen.insert(name.to_string());
                    imports.push(name.to_string());
                }
            }
        }

        imports
    }

    /// Validate imports against allowed_modules (strict mode) or allow all.
    /// Returns (valid, unapproved_imports).
    fn validate_dependencies(&self, imports: &[String]) -> (bool, Vec<String>) {
        match self.mode {
            ImportMode::AllowAll => (true, vec![]),
            ImportMode::Strict => {
                let unapproved: Vec<String> = imports
                    .iter()
                    .filter(|m| !self.is_stdlib(m) && !self.allowed_modules.contains(*m))
                    .cloned()
                    .collect();
                (unapproved.is_empty(), unapproved)
            }
        }
    }

    /// Check if a module is Python stdlib (partial heuristic, covers common ones).
    fn is_stdlib(&self, module: &str) -> bool {
        matches!(
            module,
            // Core builtins
            "sys" | "os" | "re" | "json" | "time" | "datetime" | "uuid"
            | "random" | "math" | "statistics" | "copy" | "functools"
            | "itertools" | "collections" | "abc" | "typing" | "types"
            | "pathlib" | "urllib" | "http" | "html" | "xml" | "csv"
            | "io" | "errno" | "gc" | "threading" | "multiprocessing"
            | "subprocess" | "signal" | "socket" | "ssl" | "struct"
            | "unittest" | "doctest" | "traceback" | "ast" | "dis"
            // Common extras
            | "hashlib" | "hmac" | "base64" | "binascii" | "zipfile"
            | "tarfile" | "gzip" | "bz2" | "lzma" | "configparser"
            | "argparse" | "getopt" | "logging" | "warnings" | "inspect"
            | "platform" | "ctypes" | "cProfile" | "pstats" | "shelve"
            | "pickle" | "dbm" | "sqlite3" | "decimal" | "fractions"
            | "array" | "bisect" | "heapq" | "queue" | "sched"
            | "contextvars" | "dataclasses" | "secrets" | "tomllib"
        )
    }

    /// Extract numeric sort key from filename (e.g. "01_build.py" → 1).
    fn extract_sort_key(filename: &str) -> i32 {
        let stem = std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename);
        let leading: String = stem
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '_')
            .filter(|c| c.is_ascii_digit())
            .collect();
        leading.parse::<i32>().unwrap_or(9999)
    }

    /// Extract job name from filename (e.g. "01_build.py" → "build").
    fn extract_job_name(filename: &str) -> String {
        let stem = std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename);
        // Strip leading digits and underscores: "01_build" → "build"
        stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '_')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_requirements() {
        let content = r#"
flask>=2.0
requests==2.28.0
sqlalchemy<1.4
djangoChannels>=4.0
-e git+https://github.com/foo/bar.git
# comment
-r other.txt
--index-url https://example.com
"#;
        let modules = ScriptScanner::parse_requirements(content);
        assert!(modules.contains("flask"));
        assert!(modules.contains("requests"));
        assert!(modules.contains("sqlalchemy"));
        assert!(modules.contains("djangochannels")); // lowercased
        // comments and flags ignored
        assert!(!modules.contains("other"));
    }

    #[test]
    fn test_extract_imports() {
        let source = r#"
import os
import sys
from pathlib import Path
from typing import List, Dict
import requests as http
import myproject.internal
from collections import OrderedDict
"#;
        let scanner = ScriptScanner::new(ImportMode::AllowAll, "");
        let imports = scanner.extract_imports(source);
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"sys".to_string()));
        assert!(imports.contains(&"pathlib".to_string()));
        assert!(imports.contains(&"typing".to_string()));
        assert!(imports.contains(&"requests".to_string()));
        assert!(imports.contains(&"collections".to_string()));
        assert!(imports.contains(&"myproject".to_string()));
        assert_eq!(imports.iter().filter(|m| *m == "os").count(), 1); // deduped
    }

    #[test]
    fn test_strict_mode_validation() {
        let req = "flask\nrequests\n";
        let scanner = ScriptScanner::new(ImportMode::Strict, req);

        let (valid, unapproved) = scanner.validate_dependencies(&["flask".to_string(), "requests".to_string(), "dangerous".to_string()]);
        assert!(!valid); // dangerous not in requirements
        assert!(!unapproved.is_empty()); // dangerous is unapproved
        assert!(unapproved.contains(&"dangerous".to_string()));
    }

    #[test]
    fn test_sort_key_and_name() {
        assert_eq!(ScriptScanner::extract_sort_key("01_build.py"), 1);
        assert_eq!(ScriptScanner::extract_sort_key("10_test.py"), 10);
        assert_eq!(ScriptScanner::extract_sort_key("build.py"), 9999);

        assert_eq!(ScriptScanner::extract_job_name("01_build.py"), "build");
        assert_eq!(ScriptScanner::extract_job_name("10_test.py"), "test");
        assert_eq!(ScriptScanner::extract_job_name("build.py"), "build");
        assert_eq!(ScriptScanner::extract_job_name("__init__.py"), "init__");
    }
}
