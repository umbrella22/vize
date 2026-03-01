//! Rename refactoring provider.
//!
//! Provides rename functionality for:
//! - Template bindings (variables, functions, etc.)
//! - Script identifiers
//! - CSS variables in v-bind()
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::collections::HashMap;

use tower_lsp::lsp_types::{Position, PrepareRenameResponse, Range, TextEdit, WorkspaceEdit};

use super::IdeContext;

/// Rename service for identifier renaming across SFC.
pub struct RenameService;

impl RenameService {
    /// Check if rename is valid at the given position.
    pub fn prepare_rename(ctx: &IdeContext) -> Option<PrepareRenameResponse> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset)?;

        if word.is_empty() {
            return None;
        }

        // Check if it's a renameable identifier
        if !Self::is_renameable(&word, ctx) {
            return None;
        }

        // Get the range of the word
        let (start, end) = Self::get_word_range(&ctx.content, ctx.offset)?;
        let range = Self::offset_range_to_lsp(&ctx.content, start, end);

        Some(PrepareRenameResponse::Range(range))
    }

    /// Perform rename operation.
    pub fn rename(ctx: &IdeContext, new_name: &str) -> Option<WorkspaceEdit> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset)?;

        if word.is_empty() || !Self::is_valid_identifier(new_name) {
            return None;
        }

        // Find all occurrences across the SFC
        let edits = Self::find_all_occurrences(ctx, &word);

        if edits.is_empty() {
            return None;
        }

        // Create text edits
        let text_edits: Vec<TextEdit> = edits
            .into_iter()
            .map(|(start, end)| {
                let range = Self::offset_range_to_lsp(&ctx.content, start, end);
                TextEdit {
                    range,
                    new_text: new_name.to_string(),
                }
            })
            .collect();

        let mut changes = HashMap::new();
        changes.insert(ctx.uri.clone(), text_edits);

        Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Check if the identifier is renameable.
    fn is_renameable(word: &str, ctx: &IdeContext) -> bool {
        // Don't rename Vue directives
        if word.starts_with("v-") {
            return false;
        }

        // Don't rename keywords
        if Self::is_keyword(word) {
            return false;
        }

        // Don't rename $ globals
        if word.starts_with('$') && Self::is_vue_global(word) {
            return false;
        }

        // Check if it's defined in the script
        if let Some(ref virtual_docs) = ctx.virtual_docs {
            if let Some(ref script_setup) = virtual_docs.script_setup {
                let bindings =
                    crate::virtual_code::extract_simple_bindings(&script_setup.content, true);
                if bindings.iter().any(|b| b == word) {
                    return true;
                }
            }
            if let Some(ref script) = virtual_docs.script {
                let bindings = crate::virtual_code::extract_simple_bindings(&script.content, false);
                if bindings.iter().any(|b| b == word) {
                    return true;
                }
            }
        }

        // Allow renaming any valid identifier in template context
        Self::is_valid_identifier(word)
    }

    /// Find all occurrences of an identifier in the SFC.
    fn find_all_occurrences(ctx: &IdeContext, word: &str) -> Vec<(usize, usize)> {
        let mut occurrences = Vec::new();

        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string().into(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&ctx.content, options) else {
            return occurrences;
        };

        // Find in template
        if let Some(ref template) = descriptor.template {
            let template_start = template.loc.start;
            for (offset, len) in Self::find_identifier_occurrences(&template.content, word) {
                occurrences.push((template_start + offset, template_start + offset + len));
            }
        }

        // Find in script setup
        if let Some(ref script_setup) = descriptor.script_setup {
            let script_start = script_setup.loc.start;
            for (offset, len) in Self::find_identifier_occurrences(&script_setup.content, word) {
                occurrences.push((script_start + offset, script_start + offset + len));
            }
        }

        // Find in script
        if let Some(ref script) = descriptor.script {
            let script_start = script.loc.start;
            for (offset, len) in Self::find_identifier_occurrences(&script.content, word) {
                occurrences.push((script_start + offset, script_start + offset + len));
            }
        }

        // Find in styles (v-bind usage)
        for style in &descriptor.styles {
            let style_start = style.loc.start;
            for (offset, len) in Self::find_vbind_occurrences(&style.content, word) {
                occurrences.push((style_start + offset, style_start + offset + len));
            }
        }

        // Sort by offset and deduplicate
        occurrences.sort_by_key(|(start, _)| *start);
        occurrences.dedup();

        occurrences
    }

    /// Find all occurrences of an identifier in text.
    fn find_identifier_occurrences(text: &str, word: &str) -> Vec<(usize, usize)> {
        let mut occurrences = Vec::new();
        let bytes = text.as_bytes();
        let word_len = word.len();

        let mut pos = 0;
        while let Some(found) = text[pos..].find(word) {
            let abs_pos = pos + found;

            // Check word boundaries
            let before_ok = abs_pos == 0 || !Self::is_ident_char(bytes[abs_pos - 1] as char);
            let after_ok = abs_pos + word_len >= bytes.len()
                || !Self::is_ident_char(bytes[abs_pos + word_len] as char);

            if before_ok && after_ok {
                occurrences.push((abs_pos, word_len));
            }

            pos = abs_pos + 1;
        }

        occurrences
    }

    /// Find v-bind() occurrences in CSS.
    fn find_vbind_occurrences(css: &str, word: &str) -> Vec<(usize, usize)> {
        let mut occurrences = Vec::new();
        let pattern = "v-bind(";

        let mut pos = 0;
        while let Some(start) = css[pos..].find(pattern) {
            let abs_start = pos + start + pattern.len();

            // Find the closing paren
            if let Some(end) = css[abs_start..].find(')') {
                let content = css[abs_start..abs_start + end].trim();

                // Remove quotes if present
                let var_name = content.trim_matches(|c| c == '"' || c == '\'');

                if var_name == word {
                    // Calculate the actual position of the variable name
                    let name_start = abs_start + content.find(var_name).unwrap_or(0);
                    occurrences.push((name_start, word.len()));
                }

                pos = abs_start + end + 1;
            } else {
                break;
            }
        }

        occurrences
    }

    /// Get the word at the given offset.
    fn get_word_at_offset(content: &str, offset: usize) -> Option<String> {
        if offset >= content.len() {
            return None;
        }

        let bytes = content.as_bytes();

        // Check if we're on an identifier character
        if !Self::is_ident_char(bytes[offset] as char) {
            return None;
        }

        let (start, end) = Self::get_word_range(content, offset)?;
        Some(content[start..end].to_string())
    }

    /// Get the range of the word at offset.
    fn get_word_range(content: &str, offset: usize) -> Option<(usize, usize)> {
        if offset >= content.len() {
            return None;
        }

        let bytes = content.as_bytes();

        if !Self::is_ident_char(bytes[offset] as char) {
            return None;
        }

        // Find start
        let mut start = offset;
        while start > 0 && Self::is_ident_char(bytes[start - 1] as char) {
            start -= 1;
        }

        // Find end
        let mut end = offset;
        while end < bytes.len() && Self::is_ident_char(bytes[end] as char) {
            end += 1;
        }

        // Verify it's a valid identifier start
        if !Self::is_ident_start(bytes[start] as char) {
            return None;
        }

        Some((start, end))
    }

    /// Convert byte offset range to LSP Range.
    fn offset_range_to_lsp(content: &str, start: usize, end: usize) -> Range {
        let start_pos = Self::offset_to_position(content, start);
        let end_pos = Self::offset_to_position(content, end);
        Range {
            start: start_pos,
            end: end_pos,
        }
    }

    /// Convert byte offset to LSP Position.
    fn offset_to_position(content: &str, offset: usize) -> Position {
        let mut line = 0u32;
        let mut col = 0u32;
        let mut current = 0;

        for ch in content.chars() {
            if current >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            current += ch.len_utf8();
        }

        Position {
            line,
            character: col,
        }
    }

    /// Check if character can start an identifier.
    fn is_ident_start(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_' || c == '$'
    }

    /// Check if character can be part of an identifier.
    fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_' || c == '$'
    }

    /// Check if string is a valid identifier.
    fn is_valid_identifier(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        let mut chars = s.chars();
        let first = chars.next().unwrap();

        if !Self::is_ident_start(first) {
            return false;
        }

        chars.all(Self::is_ident_char)
    }

    /// Check if word is a JavaScript keyword.
    fn is_keyword(word: &str) -> bool {
        matches!(
            word,
            "break"
                | "case"
                | "catch"
                | "continue"
                | "debugger"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "finally"
                | "for"
                | "function"
                | "if"
                | "in"
                | "instanceof"
                | "new"
                | "return"
                | "switch"
                | "this"
                | "throw"
                | "try"
                | "typeof"
                | "var"
                | "void"
                | "while"
                | "with"
                | "class"
                | "const"
                | "enum"
                | "export"
                | "extends"
                | "import"
                | "super"
                | "implements"
                | "interface"
                | "let"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "static"
                | "yield"
                | "true"
                | "false"
                | "null"
                | "undefined"
                | "async"
                | "await"
                | "of"
        )
    }

    /// Check if word is a Vue global.
    fn is_vue_global(word: &str) -> bool {
        matches!(
            word,
            "$el"
                | "$data"
                | "$props"
                | "$attrs"
                | "$refs"
                | "$slots"
                | "$root"
                | "$parent"
                | "$emit"
                | "$forceUpdate"
                | "$nextTick"
                | "$watch"
                | "$options"
                | "$event"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::RenameService;

    #[test]
    fn test_get_word_at_offset() {
        let content = "const count = ref(0)";
        assert_eq!(
            RenameService::get_word_at_offset(content, 6),
            Some("count".to_string())
        );
        assert_eq!(
            RenameService::get_word_at_offset(content, 14),
            Some("ref".to_string())
        );
    }

    #[test]
    fn test_find_identifier_occurrences() {
        let text = "const count = count + 1; console.log(count)";
        let occurrences = RenameService::find_identifier_occurrences(text, "count");
        assert_eq!(occurrences.len(), 3);
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(RenameService::is_valid_identifier("count"));
        assert!(RenameService::is_valid_identifier("_private"));
        assert!(RenameService::is_valid_identifier("$refs"));
        assert!(!RenameService::is_valid_identifier("123abc"));
        assert!(!RenameService::is_valid_identifier(""));
    }

    #[test]
    fn test_is_keyword() {
        assert!(RenameService::is_keyword("const"));
        assert!(RenameService::is_keyword("function"));
        assert!(!RenameService::is_keyword("count"));
    }

    #[test]
    fn test_find_vbind_occurrences() {
        let css = ".container { color: v-bind(textColor); width: v-bind('width'); }";
        let occurrences = RenameService::find_vbind_occurrences(css, "textColor");
        assert_eq!(occurrences.len(), 1);
    }
}
