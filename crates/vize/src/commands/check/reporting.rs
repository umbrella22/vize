//! Diagnostic reporting and source-map utilities for the check command.
//!
//! Handles mapping diagnostics from virtual TypeScript positions back to original
//! SFC line/column positions, and provides JSON output structures.

use serde::Serialize;

/// JSON output structure for `--format json`.
#[derive(Serialize)]
#[allow(clippy::disallowed_types)]
pub(crate) struct JsonOutput {
    pub files: Vec<JsonFileResult>,
    #[serde(rename = "errorCount")]
    pub error_count: usize,
    #[serde(rename = "fileCount")]
    pub file_count: usize,
}

/// Per-file result in JSON output.
#[derive(Serialize)]
#[allow(clippy::disallowed_types)]
pub(crate) struct JsonFileResult {
    pub file: String,
    #[serde(rename = "virtualTs")]
    pub virtual_ts: String,
    pub diagnostics: Vec<String>,
}

/// Convert a line/column position in the virtual TS to a line/column in the original SFC.
///
/// Steps:
/// 1. Convert virtual TS line/col to byte offset in virtual TS
/// 2. Find matching source mapping
/// 3. Compute byte offset in original SFC
/// 4. Convert SFC byte offset to line/col
pub(crate) fn map_diagnostic_position(
    virtual_ts: &str,
    source_map: &[vize_canon::virtual_ts::VizeMapping],
    original_content: &str,
    vts_line: u32,
    vts_character: u32,
) -> (u32, u32) {
    // Step 1: line/col -> byte offset in virtual TS
    let vts_offset = line_col_to_offset(virtual_ts, vts_line, vts_character);

    // Step 2: Find matching source mapping
    for mapping in source_map {
        if vts_offset >= mapping.gen_range.start && vts_offset < mapping.gen_range.end {
            // Step 3: Compute corresponding offset in original SFC
            let delta = vts_offset - mapping.gen_range.start;
            let src_offset = mapping.src_range.start + delta;
            // Clamp to source range
            let src_offset = src_offset.min(mapping.src_range.end.saturating_sub(1));

            // Step 4: Convert SFC offset to line/col (1-based)
            let (line, col) = offset_to_line_col(original_content, src_offset);
            return (line + 1, col + 1);
        }
    }

    // Fallback: return virtual TS position (1-based)
    (vts_line + 1, vts_character + 1)
}

/// Convert line/column (0-based) to byte offset in content.
fn line_col_to_offset(content: &str, line: u32, col: u32) -> usize {
    let mut current_line = 0u32;
    let mut offset = 0usize;

    for (i, ch) in content.char_indices() {
        if current_line == line {
            return i + col as usize;
        }
        if ch == '\n' {
            current_line += 1;
        }
        offset = i + ch.len_utf8();
    }

    offset + col as usize
}

/// Convert byte offset to line/column (0-based) in content.
fn offset_to_line_col(content: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;

    for (i, ch) in content.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}
