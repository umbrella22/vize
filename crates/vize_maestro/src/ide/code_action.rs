//! Code action provider for Vue SFC files.
//!
//! Provides code actions for:
//! - Lint fixes from vize_patina
//! - Quick fixes for common issues
//! - Refactoring actions
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use super::IdeContext;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range, TextEdit, WorkspaceEdit,
};

/// Code action service for providing quick fixes and refactorings.
pub struct CodeActionService;

impl CodeActionService {
    /// Get code actions for the given context and range.
    pub fn code_actions(ctx: &IdeContext, range: Range) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        // Collect lint fix actions
        actions.extend(Self::collect_lint_fixes(ctx, range));

        actions
    }

    /// Collect lint fix actions from vize_patina diagnostics.
    fn collect_lint_fixes(ctx: &IdeContext, range: Range) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        // Parse SFC to get template
        #[allow(clippy::disallowed_methods)]
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string().into(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&ctx.content, options) else {
            return actions;
        };

        let Some(ref template) = descriptor.template else {
            return actions;
        };

        // Run linter to get diagnostics with fixes
        let linter = vize_patina::Linter::new();
        let result = linter.lint_template(&template.content, ctx.uri.path());

        // Template block offset in SFC
        let template_start_line = template.loc.start_line as u32;

        for lint_diag in result.diagnostics {
            // Check if diagnostic has a fix
            let Some(ref fix) = lint_diag.fix else {
                continue;
            };

            // Convert lint diagnostic position to SFC position
            let (start_line, start_col) =
                offset_to_line_col(&template.content, lint_diag.start as usize);
            let (end_line, end_col) = offset_to_line_col(&template.content, lint_diag.end as usize);

            let diag_range = Range {
                start: Position {
                    line: template_start_line + start_line - 1,
                    character: start_col,
                },
                end: Position {
                    line: template_start_line + end_line - 1,
                    character: end_col,
                },
            };

            // Check if the diagnostic range overlaps with the requested range
            if !ranges_overlap(&diag_range, &range) {
                continue;
            }

            // Convert fix edits to LSP TextEdits
            let edits: Vec<TextEdit> = fix
                .edits
                .iter()
                .map(|edit| {
                    let (edit_start_line, edit_start_col) =
                        offset_to_line_col(&template.content, edit.start as usize);
                    let (edit_end_line, edit_end_col) =
                        offset_to_line_col(&template.content, edit.end as usize);

                    TextEdit {
                        range: Range {
                            start: Position {
                                line: template_start_line + edit_start_line - 1,
                                character: edit_start_col,
                            },
                            end: Position {
                                line: template_start_line + edit_end_line - 1,
                                character: edit_end_col,
                            },
                        },
                        #[allow(clippy::disallowed_methods)]
                        new_text: edit.new_text.to_string(),
                    }
                })
                .collect();

            // Create workspace edit
            #[allow(clippy::disallowed_types)]
            let mut changes = std::collections::HashMap::new();
            changes.insert(ctx.uri.clone(), edits);

            let workspace_edit = WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            };

            // Create code action
            #[allow(clippy::disallowed_macros)]
            let action = CodeAction {
                title: format!("Fix: {}", fix.message),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: None, // Could link to specific diagnostic
                edit: Some(workspace_edit),
                command: None,
                is_preferred: Some(true),
                disabled: None,
                data: None,
            };

            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        actions
    }

    /// Get all available fixes for a document (for "fix all" actions).
    pub fn get_all_fixes(ctx: &IdeContext) -> Option<WorkspaceEdit> {
        #[allow(clippy::disallowed_methods)]
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string().into(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;
        let template = descriptor.template.as_ref()?;

        let linter = vize_patina::Linter::new();
        let result = linter.lint_template(&template.content, ctx.uri.path());

        let template_start_line = template.loc.start_line as u32;

        let mut all_edits: Vec<TextEdit> = Vec::new();

        for lint_diag in result.diagnostics {
            if let Some(ref fix) = lint_diag.fix {
                for edit in &fix.edits {
                    let (edit_start_line, edit_start_col) =
                        offset_to_line_col(&template.content, edit.start as usize);
                    let (edit_end_line, edit_end_col) =
                        offset_to_line_col(&template.content, edit.end as usize);

                    all_edits.push(TextEdit {
                        range: Range {
                            start: Position {
                                line: template_start_line + edit_start_line - 1,
                                character: edit_start_col,
                            },
                            end: Position {
                                line: template_start_line + edit_end_line - 1,
                                character: edit_end_col,
                            },
                        },
                        #[allow(clippy::disallowed_methods)]
                        new_text: edit.new_text.to_string(),
                    });
                }
            }
        }

        if all_edits.is_empty() {
            return None;
        }

        // Sort edits by position (reverse order for safe application)
        all_edits.sort_by(|a, b| {
            b.range
                .start
                .line
                .cmp(&a.range.start.line)
                .then(b.range.start.character.cmp(&a.range.start.character))
        });

        // Remove overlapping edits (keep the first one)
        let mut filtered_edits: Vec<TextEdit> = Vec::new();
        for edit in all_edits {
            let overlaps = filtered_edits
                .iter()
                .any(|e| ranges_overlap(&e.range, &edit.range));
            if !overlaps {
                filtered_edits.push(edit);
            }
        }

        #[allow(clippy::disallowed_types)]
        let mut changes = std::collections::HashMap::new();
        changes.insert(ctx.uri.clone(), filtered_edits);

        Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }
}

/// Convert byte offset to (line, column) - both 0-indexed.
fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_offset += ch.len_utf8();
    }

    (line, col)
}

/// Check if two ranges overlap.
fn ranges_overlap(a: &Range, b: &Range) -> bool {
    // Ranges overlap if neither is completely before or after the other
    !(a.end.line < b.start.line
        || (a.end.line == b.start.line && a.end.character < b.start.character)
        || b.end.line < a.start.line
        || (b.end.line == a.start.line && b.end.character < a.start.character))
}

#[cfg(test)]
mod tests {
    use super::{offset_to_line_col, ranges_overlap};
    use tower_lsp::lsp_types::{Position, Range};

    #[test]
    fn test_ranges_overlap() {
        let a = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 10,
            },
        };
        let b = Range {
            start: Position {
                line: 0,
                character: 5,
            },
            end: Position {
                line: 0,
                character: 15,
            },
        };
        assert!(ranges_overlap(&a, &b));

        let c = Range {
            start: Position {
                line: 0,
                character: 20,
            },
            end: Position {
                line: 0,
                character: 30,
            },
        };
        assert!(!ranges_overlap(&a, &c));
    }

    #[test]
    fn test_offset_to_line_col() {
        let source = "abc\ndef\nghi";
        assert_eq!(offset_to_line_col(source, 0), (0, 0));
        assert_eq!(offset_to_line_col(source, 3), (0, 3));
        assert_eq!(offset_to_line_col(source, 4), (1, 0));
        assert_eq!(offset_to_line_col(source, 8), (2, 0));
    }
}
