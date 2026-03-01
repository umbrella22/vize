//! Helper utilities for the definition service.
//!
//! Provides word extraction, position conversion, import resolution,
//! and attribute inspection helpers.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros
)]

use std::path::PathBuf;

use tower_lsp::lsp_types::Url;

use super::IdeContext;

/// Get the word at a given offset.
pub(crate) fn get_word_at_offset(content: &str, offset: usize) -> Option<String> {
    if offset >= content.len() {
        return None;
    }

    let bytes = content.as_bytes();

    // If the character at offset is not a word character, return None
    if !is_word_char(bytes[offset]) {
        return None;
    }

    // Find word start
    let mut start = offset;
    while start > 0 {
        let c = bytes[start - 1];
        if !is_word_char(c) {
            break;
        }
        start -= 1;
    }

    // Find word end
    let mut end = offset;
    while end < bytes.len() {
        let c = bytes[end];
        if !is_word_char(c) {
            break;
        }
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(String::from_utf8_lossy(&bytes[start..end]).to_string())
}

/// Check if a byte is a valid word character.
#[inline]
pub(crate) fn is_word_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

/// Convert byte offset to (line, character) position.
pub(crate) fn offset_to_position(content: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut current_offset = 0usize;

    for ch in content.chars() {
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

/// Skip virtual code header comments.
pub(crate) fn skip_virtual_header(content: &str) -> usize {
    let mut offset = 0;
    for line in content.lines() {
        if line.starts_with("//") || line.trim().is_empty() {
            offset += line.len() + 1; // +1 for newline
        } else {
            break;
        }
    }
    offset
}

/// Get the tag name at the given offset (if cursor is on a tag).
pub(crate) fn get_tag_at_offset(content: &str, offset: usize) -> Option<String> {
    if offset >= content.len() {
        return None;
    }

    let bytes = content.as_bytes();

    // Look backwards for '<'
    let mut tag_start = None;
    let mut i = offset;
    while i > 0 {
        i -= 1;
        if bytes[i] == b'<' {
            tag_start = Some(i + 1);
            break;
        }
        if bytes[i] == b'>' || bytes[i] == b'\n' {
            break;
        }
    }

    let start = tag_start?;

    // Find the end of the tag name
    let mut end = start;
    while end < bytes.len() {
        let c = bytes[end];
        if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' {
            end += 1;
        } else {
            break;
        }
    }

    if end > start {
        Some(String::from_utf8_lossy(&bytes[start..end]).to_string())
    } else {
        None
    }
}

/// Get the attribute name and component name at the cursor position.
pub(crate) fn get_attribute_and_component_at_offset(
    ctx: &IdeContext<'_>,
) -> Option<(String, String)> {
    let content = &ctx.content;
    let offset = ctx.offset;

    // Find the start of the current line or tag
    let mut tag_start = offset;
    let mut depth = 0;

    // Scan backwards to find the opening tag
    while tag_start > 0 {
        let c = content.as_bytes()[tag_start - 1];
        if c == b'>' {
            depth += 1;
        } else if c == b'<' {
            if depth == 0 {
                break;
            }
            depth -= 1;
        }
        tag_start -= 1;
    }

    if tag_start == 0 {
        return None;
    }

    // Find the end of the tag (closing >)
    let tag_end = content[offset..].find('>')? + offset;
    let tag_content = &content[tag_start..tag_end];

    // Extract tag name
    let tag_name_end = tag_content.find(|c: char| c.is_whitespace() || c == '>' || c == '/')?;
    let tag_name = &tag_content[..tag_name_end];

    // Check if cursor is on an attribute
    let cursor_in_tag = offset - tag_start;
    let before_cursor = &tag_content[..cursor_in_tag];

    // Find the attribute we're on
    let attr_start = before_cursor.rfind(|c: char| c.is_whitespace() || c == ':' || c == '@')?;
    let after_attr_start = &before_cursor[attr_start..].trim_start();

    // Extract attribute name
    let attr_end = after_attr_start
        .find(|c: char| c == '=' || c.is_whitespace())
        .unwrap_or(after_attr_start.len());
    let mut attr_name = &after_attr_start[..attr_end];

    // Handle directive prefixes
    if let Some(stripped) = attr_name.strip_prefix(':') {
        attr_name = stripped;
    } else if let Some(stripped) = attr_name.strip_prefix("v-bind:") {
        attr_name = stripped;
    } else if attr_name.starts_with('@')
        || attr_name.starts_with("v-on:")
        || attr_name.starts_with("v-")
    {
        // Event handlers and other directives - not props
        return None;
    }

    if attr_name.is_empty() {
        return None;
    }

    Some((attr_name.to_string(), tag_name.to_string()))
}

/// Convert kebab-case to camelCase.
pub(crate) fn kebab_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Find a property name within defineProps type/object definition.
pub(crate) fn find_prop_in_define_props(content: &str, property_name: &str) -> Option<usize> {
    #[allow(clippy::disallowed_macros)]
    let patterns = [
        format!("{}: ", property_name),
        format!("{}?: ", property_name),
        format!("{} :", property_name),
        format!("{}?:", property_name),
    ];

    for pattern in &patterns {
        if let Some(pos) = content.find(pattern.as_str()) {
            let before = &content[..pos];
            let open_angle = before.matches('<').count();
            let close_angle = before.matches('>').count();
            let open_curly = before.matches('{').count();
            let close_curly = before.matches('}').count();

            if open_angle > close_angle || open_curly > close_curly {
                return Some(pos);
            }
        }
    }

    None
}

/// Check if the cursor is inside a Vue directive expression.
pub(crate) fn is_in_vue_directive_expression(ctx: &IdeContext) -> bool {
    let content = &ctx.content;
    let offset = ctx.offset;

    // Check if we're inside a mustache expression {{ ... }}
    let before = &content[..offset];
    let after = &content[offset..];

    if let Some(mustache_start) = before.rfind("{{") {
        let between = &content[mustache_start + 2..offset];
        if !between.contains("}}") && after.contains("}}") {
            return true;
        }
    }

    // Check if we're inside an attribute value
    let mut pos = offset;
    let mut in_quotes = false;
    let mut quote_char = '"';

    while pos > 0 {
        let c = content.as_bytes()[pos - 1] as char;
        if c == '"' || c == '\'' {
            in_quotes = true;
            quote_char = c;
            pos -= 1;
            break;
        }
        if c == '>' || c == '<' {
            return false;
        }
        pos -= 1;
    }

    if !in_quotes {
        return false;
    }

    // Skip the = sign
    while pos > 0 && content.as_bytes()[pos - 1] == b'=' {
        pos -= 1;
    }

    // Get the attribute name by scanning backwards
    let attr_end = pos;
    while pos > 0 {
        let c = content.as_bytes()[pos - 1] as char;
        if c.is_whitespace() || c == '<' || c == '>' {
            break;
        }
        pos -= 1;
    }

    let attr_name = &content[pos..attr_end];

    if attr_name.starts_with(':')
        || attr_name.starts_with('@')
        || attr_name.starts_with('#')
        || attr_name.starts_with("v-")
    {
        let quote_start = attr_end + 1;
        if let Some(quote_end) = content[quote_start + 1..].find(quote_char) {
            let abs_quote_end = quote_start + 1 + quote_end;
            return offset <= abs_quote_end;
        }
    }

    false
}

/// Find the import path for a given component name.
pub(crate) fn find_import_path(ctx: &IdeContext<'_>, component_name: &str) -> Option<String> {
    let content = &ctx.content;

    // Pattern 1: import ComponentName from 'path'
    #[allow(clippy::disallowed_macros)]
    let default_import_pattern = format!("import {} from", component_name);
    if let Some(pos) = content.find(&default_import_pattern) {
        return extract_import_path_from_pos(content, pos + default_import_pattern.len());
    }

    // Pattern 2: import { ComponentName } from 'path'
    let import_positions: Vec<_> = content.match_indices("import ").collect();
    #[allow(clippy::disallowed_macros)]
    for (pos, _) in import_positions {
        let rest = &content[pos..];
        if let Some(from_pos) = rest.find(" from") {
            let import_clause = &rest[7..from_pos]; // Skip "import "
            if import_clause.contains(&format!("{{ {}", component_name))
                || import_clause.contains(&format!("{} }}", component_name))
                || import_clause.contains(&format!(", {}", component_name))
                || import_clause.contains(&format!("{},", component_name))
                || import_clause == format!("{{ {} }}", component_name)
            {
                return extract_import_path_from_pos(rest, from_pos + 5);
            }
        }
    }

    None
}

/// Extract import path from a position after 'from'.
pub(crate) fn extract_import_path_from_pos(content: &str, pos: usize) -> Option<String> {
    let rest = content[pos..].trim_start();

    let quote_char = rest.chars().next()?;
    if quote_char != '\'' && quote_char != '"' {
        return None;
    }

    let path_start = 1;
    let path_end = rest[path_start..].find(quote_char)?;

    Some(rest[path_start..path_start + path_end].to_string())
}

/// Resolve an import path relative to the current file.
pub(crate) fn resolve_import_path(current_uri: &Url, import_path: &str) -> Option<PathBuf> {
    let current_path = PathBuf::from(current_uri.path());
    let current_dir = current_path.parent()?;

    if import_path.starts_with("./") || import_path.starts_with("../") {
        let resolved = current_dir.join(import_path);

        if !resolved.exists() {
            let extensions = [".vue", ".ts", ".tsx", ".js", ".jsx"];
            for ext in extensions {
                let with_ext = resolved.with_extension(&ext[1..]);
                if with_ext.exists() {
                    return Some(with_ext);
                }
            }
            // Try index files
            for ext in extensions {
                #[allow(clippy::disallowed_macros)]
                let index_name = format!("index{}", ext);
                let index_file = resolved.join(index_name);
                if index_file.exists() {
                    return Some(index_file);
                }
            }
        }

        Some(resolved.canonicalize().unwrap_or(resolved))
    } else {
        None
    }
}
