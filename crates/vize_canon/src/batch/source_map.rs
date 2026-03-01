//! Source map for mapping virtual TypeScript positions to SFC positions.

use super::import_rewriter::ImportSourceMap;
use super::SfcBlockType;

/// Source map for SFC to virtual TypeScript mapping.
#[derive(Debug, Default)]
pub struct SfcSourceMap {
    /// Mappings from virtual TS offset to SFC position.
    mappings: Vec<SfcMapping>,
}

/// A single mapping entry.
#[derive(Debug, Clone)]
pub struct SfcMapping {
    /// Start offset in virtual TS.
    pub virtual_start: u32,
    /// End offset in virtual TS.
    pub virtual_end: u32,
    /// Start offset in SFC.
    pub sfc_start: u32,
    /// Block type in SFC.
    pub block_type: SfcBlockType,
}

impl SfcSourceMap {
    /// Create a new SFC source map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a mapping.
    pub fn add_mapping(
        &mut self,
        virtual_start: u32,
        virtual_end: u32,
        sfc_start: u32,
        block_type: SfcBlockType,
    ) {
        self.mappings.push(SfcMapping {
            virtual_start,
            virtual_end,
            sfc_start,
            block_type,
        });
    }

    /// Get the original SFC position from a virtual TS offset.
    pub fn get_original_position(&self, virtual_offset: u32) -> Option<(u32, u32, SfcBlockType)> {
        for mapping in &self.mappings {
            if virtual_offset >= mapping.virtual_start && virtual_offset < mapping.virtual_end {
                let delta = virtual_offset - mapping.virtual_start;
                let sfc_offset = mapping.sfc_start + delta;
                // For now, return offset as line (we'll convert later)
                return Some((sfc_offset, 0, mapping.block_type));
            }
        }
        None
    }

    /// Get the virtual TS offset from an SFC offset.
    pub fn get_virtual_offset(&self, sfc_offset: u32, block_type: SfcBlockType) -> Option<u32> {
        for mapping in &self.mappings {
            if mapping.block_type == block_type {
                let mapping_sfc_end =
                    mapping.sfc_start + (mapping.virtual_end - mapping.virtual_start);
                if sfc_offset >= mapping.sfc_start && sfc_offset < mapping_sfc_end {
                    let delta = sfc_offset - mapping.sfc_start;
                    return Some(mapping.virtual_start + delta);
                }
            }
        }
        None
    }
}

/// Composite source map combining import rewrites and SFC mapping.
#[derive(Debug, Default)]
pub struct CompositeSourceMap {
    /// Source map for SFC blocks (only for .vue files).
    pub sfc_map: Option<SfcSourceMap>,
    /// Source map for import rewrites.
    pub import_map: ImportSourceMap,
}

impl CompositeSourceMap {
    /// Create a new composite source map.
    pub fn new(sfc_map: Option<SfcSourceMap>, import_map: ImportSourceMap) -> Self {
        Self {
            sfc_map,
            import_map,
        }
    }

    /// Create an empty composite source map.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Get the original position from a virtual position.
    ///
    /// The mapping order is:
    /// 1. Import rewrite mapping (virtual TS -> TS with original imports)
    /// 2. SFC mapping (TS -> SFC position)
    pub fn get_original_position(
        &self,
        virtual_offset: u32,
    ) -> Option<(u32, u32, Option<SfcBlockType>)> {
        // First, reverse import rewrites
        let after_import = self.import_map.get_original_offset(virtual_offset);

        // Then, map through SFC source map if present
        if let Some(ref sfc_map) = self.sfc_map {
            if let Some((line, col, block)) = sfc_map.get_original_position(after_import) {
                return Some((line, col, Some(block)));
            }
        }

        // For .ts files without SFC map, return the import-adjusted position
        Some((after_import, 0, None))
    }
}

/// Convert byte offset to line and column (0-based).
pub fn offset_to_line_col(content: &str, offset: u32) -> Option<(u32, u32)> {
    let offset = offset as usize;
    if offset > content.len() {
        return None;
    }

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

    Some((line, col))
}

/// Convert line and column to byte offset (0-based).
pub fn line_col_to_offset(content: &str, line: u32, col: u32) -> Option<u32> {
    let mut current_line = 0u32;
    let mut current_col = 0u32;
    let mut offset = 0u32;

    for ch in content.chars() {
        if current_line == line && current_col == col {
            return Some(offset);
        }
        if ch == '\n' {
            if current_line == line {
                // Column out of bounds on this line
                return None;
            }
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
        offset += ch.len_utf8() as u32;
    }

    // Handle end of file
    if current_line == line && current_col == col {
        return Some(offset);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        line_col_to_offset, offset_to_line_col, CompositeSourceMap, ImportSourceMap, SfcBlockType,
        SfcSourceMap,
    };

    #[test]
    fn test_offset_to_line_col() {
        let content = "abc\ndef\nghi";
        assert_eq!(offset_to_line_col(content, 0), Some((0, 0)));
        assert_eq!(offset_to_line_col(content, 3), Some((0, 3)));
        assert_eq!(offset_to_line_col(content, 4), Some((1, 0)));
        assert_eq!(offset_to_line_col(content, 8), Some((2, 0)));
    }

    #[test]
    fn test_line_col_to_offset() {
        let content = "abc\ndef\nghi";
        assert_eq!(line_col_to_offset(content, 0, 0), Some(0));
        assert_eq!(line_col_to_offset(content, 0, 3), Some(3));
        assert_eq!(line_col_to_offset(content, 1, 0), Some(4));
        assert_eq!(line_col_to_offset(content, 2, 0), Some(8));
    }

    #[test]
    fn test_sfc_source_map() {
        let mut map = SfcSourceMap::new();
        // Script setup block: virtual 100-200 maps to SFC 50-150
        map.add_mapping(100, 200, 50, SfcBlockType::ScriptSetup);

        // Virtual offset 150 should map to SFC offset 100
        let result = map.get_original_position(150);
        assert!(result.is_some());
        let (offset, _, block) = result.unwrap();
        assert_eq!(offset, 100);
        assert_eq!(block, SfcBlockType::ScriptSetup);
    }

    #[test]
    fn test_composite_source_map() {
        let sfc_map = SfcSourceMap::new();
        let import_map = ImportSourceMap::empty();
        let composite = CompositeSourceMap::new(Some(sfc_map), import_map);

        // Should return position even without mappings
        let result = composite.get_original_position(50);
        assert!(result.is_some());
    }
}
