//! Vapor IR transformation.
//!
//! Transforms the template AST into Vapor IR for code generation.

mod context;
mod control;
mod directive;
mod element;
mod text;

use vize_carton::{Bump, String, Vec};

use crate::ir::{BlockIRNode, RootIRNode};
use vize_atelier_core::{RootNode, TemplateChildNode};

use context::TransformContext;
use control::{transform_for_node, transform_if_node};
use element::transform_element;
use text::{transform_interpolation, transform_text};

/// Transform AST to Vapor IR
pub fn transform_to_ir<'a>(allocator: &'a Bump, root: &RootNode<'a>) -> RootIRNode<'a> {
    let mut ctx = TransformContext::new(allocator);

    // Create block for root
    let block = transform_children(&mut ctx, &root.children);

    RootIRNode {
        node: RootNode::new(allocator, ""),
        source: String::from(""),
        template: Default::default(),
        template_index_map: Default::default(),
        root_template_indexes: Vec::new_in(allocator),
        component: Vec::new_in(allocator),
        directive: Vec::new_in(allocator),
        block,
        has_template_ref: false,
        has_deferred_v_show: false,
        templates: ctx.templates,
        element_template_map: ctx.element_template_map,
    }
}

/// Transform children nodes
fn transform_children<'a>(
    ctx: &mut TransformContext<'a>,
    children: &[TemplateChildNode<'a>],
) -> BlockIRNode<'a> {
    let mut block = BlockIRNode::new(ctx.allocator);
    // Note: Don't consume an ID for the block itself - element IDs should start from 0

    for child in children {
        match child {
            TemplateChildNode::Element(el) => {
                transform_element(ctx, el, &mut block);
            }
            TemplateChildNode::Text(text) => {
                transform_text(ctx, text, &mut block);
            }
            TemplateChildNode::Interpolation(interp) => {
                transform_interpolation(ctx, interp, &mut block);
            }
            TemplateChildNode::If(if_node) => {
                transform_if_node(ctx, if_node, &mut block);
            }
            TemplateChildNode::For(for_node) => {
                transform_for_node(ctx, for_node, &mut block);
            }
            TemplateChildNode::Comment(_) => {
                // Comments are ignored in Vapor mode
            }
            _ => {}
        }
    }

    block
}

#[cfg(test)]
mod tests {
    use super::transform_to_ir;
    use vize_atelier_core::parser::parse;
    use vize_carton::Bump;

    #[test]
    fn test_transform_simple_element() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, "<div>hello</div>");
        let ir = transform_to_ir(&allocator, &root);

        assert!(!ir.block.returns.is_empty());
    }

    #[test]
    fn test_transform_nested_elements() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, "<div><span>nested</span></div>");
        let ir = transform_to_ir(&allocator, &root);

        assert!(!ir.block.returns.is_empty());
    }
}
