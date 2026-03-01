//! Type resolution for inline script compilation.
//!
//! Handles resolving type references (interface names, type aliases, intersection types)
//! to their concrete definitions for `defineProps<TypeRef>()` patterns.

use vize_carton::{String, ToCompactString};
/// Resolve type args that may be interface/type alias references.
/// For `defineProps<Props>()` where `Props` is an interface name, resolves to the interface body.
/// For intersection types like `BaseProps & ExtendedProps`, merges all interface bodies.
/// For inline types like `{ msg: string }`, returns as-is.
pub(crate) fn resolve_type_args(
    type_args: &str,
    interfaces: &vize_carton::FxHashMap<String, String>,
    type_aliases: &vize_carton::FxHashMap<String, String>,
) -> String {
    let content = type_args.trim();

    // Already an inline object type
    if content.starts_with('{') {
        return content.to_compact_string();
    }

    // Handle intersection types: BaseProps & ExtendedProps
    if content.contains('&') {
        let parts: Vec<&str> = content.split('&').collect();
        let mut merged_props = Vec::new();
        for part in parts {
            let resolved = resolve_single_type_ref(part.trim(), interfaces, type_aliases);
            if let Some(body) = resolved {
                let body = body.trim();
                let inner = if body.starts_with('{') && body.ends_with('}') {
                    &body[1..body.len() - 1]
                } else {
                    body
                };
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    merged_props.push(trimmed.to_compact_string());
                }
            }
        }
        if !merged_props.is_empty() {
            let joined = merged_props
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            let mut result = String::with_capacity(joined.len() + 4);
            result.push_str("{ ");
            result.push_str(&joined);
            result.push_str(" }");
            return result;
        }
        return content.to_compact_string();
    }

    // Single type reference
    if let Some(body) = resolve_single_type_ref(content, interfaces, type_aliases) {
        let body = body.trim();
        if body.starts_with('{') {
            return body.to_compact_string();
        }
        let mut result = String::with_capacity(body.len() + 4);
        result.push_str("{ ");
        result.push_str(body);
        result.push_str(" }");
        return result;
    }

    // Unresolvable - return as-is
    content.to_compact_string()
}

/// Resolve a single type name to its definition body.
pub(crate) fn resolve_single_type_ref(
    name: &str,
    interfaces: &vize_carton::FxHashMap<String, String>,
    type_aliases: &vize_carton::FxHashMap<String, String>,
) -> Option<String> {
    // Strip generic params: Props<T> -> Props
    let base_name = if let Some(idx) = name.find('<') {
        name[..idx].trim()
    } else {
        name.trim()
    };

    if let Some(body) = interfaces.get(base_name) {
        return Some(body.clone());
    }
    if let Some(body) = type_aliases.get(base_name) {
        return Some(body.clone());
    }
    None
}
