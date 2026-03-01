//! Helper functions for SFC compilation.

use vize_carton::{String, ToCompactString};
/// Generate scope ID from filename
pub(super) fn generate_scope_id(filename: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    filename.hash(&mut hasher);
    let value = hasher.finish() & 0xFFFFFFFF;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(8);
    for shift in (0..32).step_by(4).rev() {
        let digit = ((value >> shift) & 0xF) as usize;
        out.push(HEX[digit] as char);
    }
    out
}

/// Extract component name from filename
pub(super) fn extract_component_name(filename: &str) -> String {
    std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("anonymous")
        .to_compact_string()
}
