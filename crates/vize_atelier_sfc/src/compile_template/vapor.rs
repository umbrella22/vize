//! Vapor mode template compilation.

use vize_atelier_vapor::{compile_vapor, VaporCompilerOptions};
use vize_carton::{Bump, String, ToCompactString};

use crate::types::{SfcError, SfcTemplateBlock};

/// Compile template block using Vapor mode
pub(crate) fn compile_template_block_vapor(
    template: &SfcTemplateBlock,
    scope_id: &str,
    has_scoped: bool,
) -> Result<String, SfcError> {
    let allocator = Bump::new();

    // Build Vapor compiler options
    let vapor_opts = VaporCompilerOptions {
        prefix_identifiers: false,
        ssr: false,
        ..Default::default()
    };

    // Compile template with Vapor
    let result = compile_vapor(&allocator, &template.content, vapor_opts);

    if !result.error_messages.is_empty() {
        let mut message = String::from("Vapor template compilation errors: ");
        use std::fmt::Write as _;
        let _ = write!(&mut message, "{:?}", result.error_messages);
        return Err(SfcError {
            message,
            code: Some("VAPOR_TEMPLATE_ERROR".to_compact_string()),
            loc: Some(template.loc.clone()),
        });
    }

    // Process the Vapor output to extract imports and render function
    let mut output = String::default();
    let scope_attr = if has_scoped {
        let mut attr = String::with_capacity(scope_id.len() + 7);
        attr.push_str("data-v-");
        attr.push_str(scope_id);
        attr
    } else {
        String::default()
    };

    // Parse the Vapor output to separate imports and function body
    let code = &result.code;

    // Extract import line
    if let Some(import_end) = code.find('\n') {
        let import_line = &code[..import_end];
        // Rewrite import to use 'vue' instead of 'vue/vapor' for compatibility
        output.push_str(import_line);
        output.push('\n');

        // Extract template declarations and function body
        let rest = &code[import_end + 1..];

        // Find template declarations (const tN = ...)
        let mut template_decls = Vec::new();
        let mut func_start = 0;
        for (i, line) in rest.lines().enumerate() {
            if line.starts_with("const t") && line.contains("_template(") {
                // Add scope ID to template if scoped
                if has_scoped && !scope_attr.is_empty() {
                    let modified = add_scope_id_to_template(line, &scope_attr);
                    template_decls.push(modified);
                } else {
                    template_decls.push(line.to_compact_string());
                }
            } else if line.starts_with("export default") {
                func_start = i;
                break;
            }
        }

        // Output template declarations
        for decl in template_decls {
            output.push_str(&decl);
            output.push('\n');
        }

        // Extract and convert the function body
        let lines: Vec<&str> = rest.lines().collect();
        if func_start < lines.len() {
            // Convert "export default () => {" to "function render(_ctx, $props, $emit, $attrs, $slots) {"
            output.push_str("function render(_ctx, $props, $emit, $attrs, $slots) {\n");

            // Copy function body (skip "export default () => {" and final "}")
            for line in lines.iter().skip(func_start + 1) {
                if *line == "}" {
                    break;
                }
                output.push_str(line);
                output.push('\n');
            }

            output.push_str("}\n");
        }
    }

    Ok(output)
}

/// Add scope ID to template string
pub(super) fn add_scope_id_to_template(template_line: &str, scope_id: &str) -> String {
    // Find the template string content and add scope_id to the first element
    if let Some(start) = template_line.find("\"<") {
        if let Some(end) = template_line.rfind(">\"") {
            let prefix = &template_line[..start + 2]; // up to and including "<"
            let content = &template_line[start + 2..end + 1]; // element content
            let suffix = &template_line[end + 1..]; // closing quote and paren

            // Find end of first tag name
            if let Some(tag_end) = content.find(|c: char| c.is_whitespace() || c == '>') {
                let tag_name = &content[..tag_end];
                let rest = &content[tag_end..];

                // Insert scope_id attribute after tag name
                let mut result = String::with_capacity(
                    prefix.len() + tag_name.len() + scope_id.len() + rest.len() + suffix.len() + 1,
                );
                result.push_str(prefix);
                result.push_str(tag_name);
                result.push(' ');
                result.push_str(scope_id);
                result.push_str(rest);
                result.push_str(suffix);
                return result;
            }
        }
    }
    template_line.to_compact_string()
}
