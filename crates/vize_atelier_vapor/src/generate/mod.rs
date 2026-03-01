//! Vapor code generation.
//!
//! Generates JavaScript code from Vapor IR.

mod context;
mod helpers;
mod operations;
mod setup;

use std::fmt::Write;

use crate::ir::{BlockIRNode, OperationNode, RootIRNode};
use vize_carton::{FxHashMap, String, ToCompactString};

use context::GenerateContext;
use helpers::generate_effect;
use operations::generate_operation;
use setup::{collect_delegate_events, escape_template, generate_imports};

/// Vapor code generation result
pub struct VaporGenerateResult {
    /// Generated code
    pub code: String,
    /// Static templates
    pub templates: Vec<String>,
}

/// Generate Vapor code from IR
pub fn generate_vapor(ir: &RootIRNode<'_>) -> VaporGenerateResult {
    let mut ctx = GenerateContext::new(&ir.element_template_map);

    // Template helper is always used if we have templates
    if !ir.templates.is_empty() {
        ctx.use_helper("template");
    }

    // Generate template declarations (to separate string, we'll prepend imports later)
    let mut template_code = String::default();
    for (i, template) in ir.templates.iter().enumerate() {
        writeln!(
            template_code,
            "const t{} = _template(\"{}\", true)",
            i,
            escape_template(template)
        )
        .ok();
    }

    // First pass: collect delegate events
    collect_delegate_events(&mut ctx, &ir.block);

    // Generate component function body first to collect used helpers
    ctx.push_line("export function render(_ctx) {");
    ctx.indent();

    // Generate block content (includes template instantiation, text nodes, operations, effects, return)
    generate_block(&mut ctx, &ir.block, &ir.element_template_map);

    ctx.deindent();
    ctx.push_line("}");

    // Generate delegate events code (after templates, before function)
    let mut delegate_code = String::default();
    if !ctx.delegate_events.is_empty() {
        ctx.use_helper("delegateEvents");
        let mut events: Vec<_> = ctx.delegate_events.iter().collect();
        events.sort();
        for event in events {
            writeln!(delegate_code, "_delegateEvents(\"{}\")", event).ok();
        }
    }

    // Now generate imports at the front with only used helpers
    let imports = generate_imports(&ctx);

    // Combine: imports + templates + delegate events + function body
    let mut final_code = imports;
    if !template_code.is_empty() {
        final_code.push_str(&template_code);
    }
    if !delegate_code.is_empty() {
        final_code.push_str(&delegate_code);
    }
    // Add blank line before function
    if !final_code.is_empty() {
        final_code.push('\n');
    }
    final_code.push_str(&ctx.code);

    VaporGenerateResult {
        code: final_code,
        templates: ir.templates.iter().cloned().collect(),
    }
}

/// Generate block
fn generate_block(
    ctx: &mut GenerateContext,
    block: &BlockIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    // Instantiate templates for elements in this block's returns
    for element_id in block.returns.iter() {
        if let Some(&template_index) = element_template_map.get(element_id) {
            let mut line = String::with_capacity(32);
            line.push_str("const n");
            line.push_str(&element_id.to_compact_string());
            line.push_str(" = t");
            line.push_str(&template_index.to_compact_string());
            line.push_str("()");
            ctx.push_line(&line);
        }
    }

    // Generate text node references for effects in this block
    for effect in block.effect.iter() {
        for op in effect.operations.iter() {
            if let OperationNode::SetText(set_text) = op {
                ctx.use_helper("txt");
                let var_name = ctx.next_text_node(set_text.element);
                let mut line = String::with_capacity(32);
                line.push_str("const ");
                line.push_str(&var_name);
                line.push_str(" = _txt(n");
                line.push_str(&set_text.element.to_compact_string());
                line.push(')');
                ctx.push_line(&line);
            }
        }
    }

    // Generate operations
    for op in block.operation.iter() {
        generate_operation(ctx, op, element_template_map);
    }

    // Generate effects
    for effect in block.effect.iter() {
        generate_effect(ctx, effect, element_template_map);
    }

    // Generate return
    if !block.returns.is_empty() {
        let returns = block
            .returns
            .iter()
            .map(|r| ["n", &r.to_compact_string()].concat())
            .collect::<Vec<_>>()
            .join(", ");

        if block.returns.len() == 1 {
            ctx.push_line(&["return ", &returns].concat());
        } else {
            ctx.push_line(&["return [", &returns, "]"].concat());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_vapor, setup::escape_template};
    use crate::transform::transform_to_ir;
    use vize_atelier_core::parser::parse;
    use vize_carton::Bump;

    #[test]
    fn test_generate_simple() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, "<div>hello</div>");
        let ir = transform_to_ir(&allocator, &root);
        let result = generate_vapor(&ir);

        assert!(!result.code.is_empty());
        assert!(result.code.contains("export function render"));
    }

    #[test]
    fn test_generate_with_event() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, r#"<button @click="handleClick">Click</button>"#);
        let ir = transform_to_ir(&allocator, &root);
        let result = generate_vapor(&ir);

        assert!(result.code.contains("createInvoker"));
        assert!(result.code.contains("click"));
    }

    #[test]
    fn test_escape_template() {
        assert_eq!(escape_template("hello"), "hello");
        assert_eq!(escape_template("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_template("hello\"world"), "hello\\\"world");
    }
}
