//! Props type generation for virtual TypeScript.
//!
//! Generates `Props` type definitions and template-level prop variable
//! declarations from Vue SFC macro analysis.

use vize_carton::append;
use vize_carton::cstr;
use vize_carton::String;
use vize_croquis::Croquis;

/// Generate Props type definition at module level.
pub(crate) fn generate_props_type(ts: &mut String, summary: &Croquis) {
    let props = summary.macros.props();
    let has_props = !props.is_empty();
    let define_props_type_args = summary
        .macros
        .define_props()
        .and_then(|m| m.type_args.as_ref());
    let props_already_defined = summary
        .type_exports
        .iter()
        .any(|te| te.name.as_str() == "Props");

    ts.push_str("// ========== Exported Types ==========\n");

    if props_already_defined {
        // User defined Props, no need to re-export
    } else if let Some(type_args) = define_props_type_args {
        let inner_type = type_args
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .unwrap_or(type_args.as_str());
        let is_simple_reference = inner_type
            .chars()
            .all(|c: char| c.is_alphanumeric() || c == '_');
        if is_simple_reference
            && summary
                .type_exports
                .iter()
                .any(|te| te.name.as_str() == inner_type)
        {
            // Type arg references existing type
        } else {
            append!(*ts, "export type Props = {inner_type};\n");
        }
    } else if has_props {
        ts.push_str("export type Props = {\n");
        for prop in props {
            let prop_type = prop.prop_type.as_deref().unwrap_or("unknown");
            let optional = if prop.required { "" } else { "?" };
            append!(*ts, "  {}{optional}: {prop_type};\n", prop.name);
        }
        ts.push_str("};\n");
    } else {
        ts.push_str("export type Props = {};\n");
    }

    ts.push('\n');
}

/// Generate props variables inside template closure.
pub(crate) fn generate_props_variables(
    ts: &mut String,
    summary: &Croquis,
    script_content: Option<&str>,
) {
    let props = summary.macros.props();
    let has_props = !props.is_empty();
    let define_props_type_args = summary
        .macros
        .define_props()
        .and_then(|m| m.type_args.as_ref());

    if has_props || define_props_type_args.is_some() {
        ts.push_str("  // Props are available in template as variables\n");
        ts.push_str("  // Access via `propName` or `props.propName`\n");
        ts.push_str("  const props: Props = {} as Props;\n");
        ts.push_str("  void props; // Mark as used to avoid TS6133\n");

        if has_props {
            // Runtime-declared props: generate individual variables
            for prop in props {
                append!(*ts, "  const {} = props[\"{}\"];\n", prop.name, prop.name);
                append!(*ts, "  void {};\n", prop.name);
            }
        } else if let Some(type_args) = define_props_type_args {
            // Type-only defineProps<TypeName>(): extract fields
            // type_args may include angle brackets (e.g., "<Props>"), strip them
            let type_name = type_args
                .trim()
                .strip_prefix('<')
                .and_then(|s| s.strip_suffix('>'))
                .unwrap_or(type_args.trim());

            // Try TypeResolver first (handles inline object types and registered types)
            let type_properties = summary.types.extract_properties(type_name);
            if !type_properties.is_empty() {
                for prop in &type_properties {
                    append!(*ts, "  const {} = props[\"{}\"];\n", prop.name, prop.name);
                    append!(*ts, "  void {};\n", prop.name);
                }
            } else if let Some(script) = script_content {
                // Fallback: extract field names from script text (for local interfaces)
                let field_names = extract_interface_fields(script, type_name);
                for field in &field_names {
                    append!(*ts, "  const {field} = props[\"{field}\"];\n");
                    append!(*ts, "  void {field};\n");
                }
            }
        }
        ts.push('\n');
    }
}

/// Extract field names from an interface or type literal in script content.
/// Fallback for when TypeResolver doesn't have the type registered.
pub(crate) fn extract_interface_fields(script: &str, type_name: &str) -> Vec<String> {
    let mut fields = Vec::new();

    let body = if type_name.starts_with('{') {
        Some(type_name)
    } else {
        find_type_body(script, type_name)
    };

    if let Some(body) = body {
        let inner = if let Some(start) = body.find('{') {
            let end = find_matching_brace(body, start);
            &body[start + 1..end]
        } else {
            body
        };

        for line in inner.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed == "}"
                || trimmed == "};"
            {
                continue;
            }
            let trimmed = trimmed.strip_prefix("readonly ").unwrap_or(trimmed);
            if let Some(colon_pos) = trimmed.find(':') {
                let field_name = trimmed[..colon_pos].trim().trim_end_matches('?');
                if !field_name.is_empty()
                    && field_name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
                {
                    fields.push(field_name.into());
                }
            }
        }
    }

    fields
}

/// Find the body of an interface or type declaration in script content.
fn find_type_body<'a>(script: &'a str, type_name: &str) -> Option<&'a str> {
    for pattern in &[
        cstr!("interface {type_name} "),
        cstr!("interface {type_name}{{"),
        cstr!("type {type_name} "),
    ] {
        if let Some(pos) = script.find(pattern.as_str()) {
            let rest = &script[pos..];
            if let Some(brace_start) = rest.find('{') {
                let end = find_matching_brace(rest, brace_start);
                return Some(&rest[..end + 1]);
            }
        }
    }
    None
}

/// Find the matching closing brace for an opening brace at `start`.
fn find_matching_brace(s: &str, start: usize) -> usize {
    let mut depth = 0;
    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return start + i;
                }
            }
            _ => {}
        }
    }
    s.len().saturating_sub(1)
}
