//! References provider for Vue SFC files.
//!
//! Provides find-all-references for:
//! - Script bindings used in template
//! - Script bindings used in other script code
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]
//! - Script bindings used in style v-bind()

mod script;
mod template;

use tower_lsp::lsp_types::Location;

use super::IdeContext;

/// References service for finding all references to a symbol.
pub struct ReferencesService;

impl ReferencesService {
    /// Find all references to the symbol at the current position.
    pub fn references(ctx: &IdeContext, include_declaration: bool) -> Option<Vec<Location>> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset)?;

        if word.is_empty() {
            return None;
        }

        let mut locations = Vec::new();

        // Find definition location if requested
        if include_declaration {
            if let Some(def_loc) = Self::find_definition_location(ctx, &word) {
                locations.push(def_loc);
            }
        }

        // Find references in template
        locations.extend(Self::find_references_in_template(ctx, &word));

        // Find references in script
        locations.extend(Self::find_references_in_script(ctx, &word));

        // Find references in style
        locations.extend(Self::find_references_in_style(ctx, &word));

        if locations.is_empty() {
            None
        } else {
            // Remove duplicates
            locations.sort_by(|a, b| {
                a.range
                    .start
                    .line
                    .cmp(&b.range.start.line)
                    .then(a.range.start.character.cmp(&b.range.start.character))
            });
            locations.dedup_by(|a, b| a.range.start == b.range.start && a.range.end == b.range.end);
            Some(locations)
        }
    }

    /// Get the word at an offset.
    fn get_word_at_offset(content: &str, offset: usize) -> Option<String> {
        if offset >= content.len() {
            return None;
        }

        let bytes = content.as_bytes();

        if !Self::is_identifier_char(bytes[offset]) {
            return None;
        }

        let mut start = offset;
        while start > 0 && Self::is_identifier_char(bytes[start - 1]) {
            start -= 1;
        }

        let mut end = offset;
        while end < bytes.len() && Self::is_identifier_char(bytes[end]) {
            end += 1;
        }

        if start == end {
            return None;
        }

        Some(String::from_utf8_lossy(&bytes[start..end]).to_string())
    }

    /// Check if a byte is an identifier character.
    #[inline]
    fn is_identifier_char(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
    }

    /// Convert offset to (line, character).
    pub(crate) fn offset_to_position(content: &str, offset: usize) -> (u32, u32) {
        let mut line = 0u32;
        let mut col = 0u32;
        let mut current = 0usize;

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

        (line, col)
    }
}

#[cfg(test)]
mod tests {
    use super::ReferencesService;

    #[test]
    fn test_find_word_occurrences() {
        let text = "message + message2 + getMessage()";

        let positions = ReferencesService::find_word_occurrences(text, "message");
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], 0);

        let positions = ReferencesService::find_word_occurrences(text, "message2");
        assert_eq!(positions.len(), 1);
    }

    #[test]
    fn test_find_identifier_references_in_script() {
        let content = r#"
const message = ref('hello')
console.log(message)
const other = message.value
"#;

        let refs = ReferencesService::find_identifier_references_in_script(content, "message");
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_find_vbind_references_in_style() {
        let content = r#"
.container {
  color: v-bind(textColor);
  background: v-bind(bgColor);
}
"#;

        let refs = ReferencesService::find_vbind_references_in_style(content, "textColor");
        assert_eq!(refs.len(), 1);

        let refs = ReferencesService::find_vbind_references_in_style(content, "bgColor");
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn test_is_in_binding_context() {
        // Inside interpolation
        assert!(ReferencesService::is_in_binding_context("{{ message }}", 3));

        // Inside directive
        assert!(ReferencesService::is_in_binding_context("v-if=\"show\"", 7));

        // Not in binding
        assert!(!ReferencesService::is_in_binding_context(
            "<div>text</div>",
            5
        ));
    }

    #[test]
    fn test_get_word_at_offset() {
        let content = "const message = ref('hello')";

        let word = ReferencesService::get_word_at_offset(content, 6);
        assert_eq!(word, Some("message".to_string()));

        let word = ReferencesService::get_word_at_offset(content, 5);
        assert_eq!(word, None); // space
    }

    #[test]
    fn test_find_binding_in_script() {
        let content = r#"// Virtual TypeScript
// Generated

const message = ref('hello')
function handleClick() {}
"#;

        let loc = ReferencesService::find_binding_in_script(content, "message");
        assert!(loc.is_some());

        let loc = ReferencesService::find_binding_in_script(content, "handleClick");
        assert!(loc.is_some());

        let loc = ReferencesService::find_binding_in_script(content, "notFound");
        assert!(loc.is_none());
    }
}
