//! tsgo CLI executor.
//!
//! This module handles executing tsgo CLI for type checking.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::error::{TsgoNotFoundError, TsgoResult};
use super::type_checker::TypeCheckResult;
use super::virtual_project::VirtualProject;
use super::Diagnostic;
use vize_carton::String;

/// tsgo CLI executor.
pub struct TsgoExecutor {
    /// Path to tsgo executable.
    tsgo_path: PathBuf,
}

impl TsgoExecutor {
    /// Create a new executor by finding tsgo.
    pub fn new(project_root: &Path) -> Result<Self, TsgoNotFoundError> {
        // 1. Try local node_modules/.bin/tsgo
        let local_tsgo = project_root.join("node_modules/.bin/tsgo");
        if local_tsgo.exists() {
            return Ok(Self {
                tsgo_path: local_tsgo,
            });
        }

        // 2. Try which::which to find in PATH
        if let Ok(global_tsgo) = which::which("tsgo") {
            return Ok(Self {
                tsgo_path: global_tsgo,
            });
        }

        // 3. Try mise shims directory
        if let Some(mise_tsgo) = Self::find_mise_tsgo() {
            return Ok(Self {
                tsgo_path: mise_tsgo,
            });
        }

        // 4. Not found
        Err(TsgoNotFoundError::new(project_root))
    }

    /// Find tsgo in mise shims directory.
    fn find_mise_tsgo() -> Option<PathBuf> {
        // Try MISE_DATA_DIR environment variable first
        let mise_data_dir = std::env::var("MISE_DATA_DIR")
            .map(PathBuf::from)
            .ok()
            .or_else(|| {
                // Default to ~/.local/share/mise
                dirs::data_local_dir().map(|d| d.join("mise"))
            })?;

        let shims_tsgo = mise_data_dir.join("shims").join("tsgo");
        if shims_tsgo.exists() {
            return Some(shims_tsgo);
        }

        // Also try XDG_DATA_HOME/mise/shims
        if let Some(xdg_data) = std::env::var("XDG_DATA_HOME").ok().map(PathBuf::from) {
            let xdg_tsgo = xdg_data.join("mise").join("shims").join("tsgo");
            if xdg_tsgo.exists() {
                return Some(xdg_tsgo);
            }
        }

        // Try home directory directly
        if let Some(home) = dirs::home_dir() {
            let home_tsgo = home.join(".local/share/mise/shims/tsgo");
            if home_tsgo.exists() {
                return Some(home_tsgo);
            }
        }

        None
    }

    /// Get the tsgo path.
    pub fn tsgo_path(&self) -> &Path {
        &self.tsgo_path
    }

    /// Run type checking on the virtual project.
    pub fn check(&self, project: &VirtualProject) -> TsgoResult<TypeCheckResult> {
        // Materialize the virtual project first
        project.materialize()?;

        // Run tsgo
        let output = Command::new(&self.tsgo_path)
            .current_dir(project.virtual_root())
            .args([
                "--project",
                "tsconfig.json",
                "--noEmit",
                "--pretty",
                "false",
            ])
            .output()?;

        // Parse output
        #[allow(clippy::disallowed_types)]
        let stderr = std::string::String::from_utf8_lossy(&output.stderr);
        let diagnostics = self.parse_tsgo_output(&stderr, project);

        let exit_code = output.status.code().unwrap_or(-1);

        Ok(TypeCheckResult {
            diagnostics,
            exit_code,
            success: output.status.success(),
        })
    }

    /// Parse tsgo CLI output into diagnostics.
    fn parse_tsgo_output(&self, output: &str, project: &VirtualProject) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // tsgo output format: file(line,col): error TSxxxx: message
        // Example: src/App.vue.ts(10,5): error TS2304: Cannot find name 'foo'.
        for line in output.lines() {
            if let Some(diag) = self.parse_diagnostic_line(line, project) {
                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    /// Parse a single diagnostic line.
    fn parse_diagnostic_line(&self, line: &str, project: &VirtualProject) -> Option<Diagnostic> {
        // Format: file(line,col): severity TScode: message
        let paren_pos = line.find('(')?;
        let colon_pos = line.find("): ")?;

        let file_path = &line[..paren_pos];
        let location = &line[paren_pos + 1..colon_pos];
        let rest = &line[colon_pos + 3..];

        // Parse location
        let (line_num, col_num) = self.parse_location(location)?;

        // Parse severity and code
        let (severity, code, message) = self.parse_message(rest)?;

        // Map virtual path to original
        let virtual_path = project.virtual_root().join(file_path);
        let original = project.map_to_original(&virtual_path, line_num - 1, col_num - 1);

        if let Some(orig) = original {
            Some(Diagnostic {
                file: orig.path,
                line: orig.line,
                column: orig.column,
                message,
                code,
                severity,
                block_type: orig.block_type,
            })
        } else {
            // Can't map, use virtual path info
            Some(Diagnostic {
                file: PathBuf::from(file_path),
                line: line_num - 1,
                column: col_num - 1,
                message,
                code,
                severity,
                block_type: None,
            })
        }
    }

    /// Parse location string "line,col".
    fn parse_location(&self, s: &str) -> Option<(u32, u32)> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 2 {
            return None;
        }
        let line = parts[0].parse().ok()?;
        let col = parts[1].parse().ok()?;
        Some((line, col))
    }

    /// Parse message part "severity TScode: message".
    fn parse_message(&self, s: &str) -> Option<(u8, Option<u32>, String)> {
        // Format: "error TS2304: message" or "warning TS2551: message"
        let severity = if s.starts_with("error") {
            1
        } else if s.starts_with("warning") {
            2
        } else {
            1 // Default to error
        };

        // Find TS code
        let code = if let Some(ts_start) = s.find("TS") {
            let code_end = s[ts_start..]
                .find(':')
                .map(|i| ts_start + i)
                .unwrap_or(s.len());
            s[ts_start + 2..code_end].parse().ok()
        } else {
            None
        };

        // Extract message
        let message: String = if let Some(msg_start) = s.find(": ") {
            s[msg_start + 2..].into()
        } else {
            s.into()
        };

        Some((severity, code, message))
    }
}

#[cfg(test)]
mod tests {
    use super::TsgoExecutor;
    use std::path::PathBuf;

    #[test]
    fn test_parse_location() {
        let executor = TsgoExecutor {
            tsgo_path: PathBuf::from("tsgo"),
        };

        assert_eq!(executor.parse_location("10,5"), Some((10, 5)));
        assert_eq!(executor.parse_location("1,1"), Some((1, 1)));
        assert_eq!(executor.parse_location("invalid"), None);
    }

    #[test]
    fn test_parse_message() {
        let executor = TsgoExecutor {
            tsgo_path: PathBuf::from("tsgo"),
        };

        let result = executor.parse_message("error TS2304: Cannot find name 'foo'.");
        assert!(result.is_some());
        let (severity, code, message) = result.unwrap();
        assert_eq!(severity, 1);
        assert_eq!(code, Some(2304));
        assert_eq!(message.as_str(), "Cannot find name 'foo'.");

        let result = executor.parse_message("warning TS2551: Did you mean 'bar'?");
        assert!(result.is_some());
        let (severity, code, message) = result.unwrap();
        assert_eq!(severity, 2);
        assert_eq!(code, Some(2551));
        assert_eq!(message.as_str(), "Did you mean 'bar'?");
    }
}
