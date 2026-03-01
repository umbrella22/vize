//! Element transformation.
//!
//! Handles `ElementNode` transformation including plain elements, components,
//! slots, and template elements. Also provides template string generation
//! and static analysis helpers.

use vize_carton::{append, cstr, Box, String, Vec};

use crate::ir::{BlockIRNode, CreateComponentIRNode, IRProp, OperationNode, SlotOutletIRNode};
use vize_atelier_core::{
    ElementNode, ElementType, ExpressionNode, PropNode, SimpleExpressionNode, SourceLocation,
    TemplateChildNode,
};

use super::{
    context::TransformContext,
    control::{transform_for_node, transform_if_node},
    directive::transform_directive,
    text::{transform_interpolation, transform_text, transform_text_children},
};
/// Transform element node
pub(crate) fn transform_element<'a>(
    ctx: &mut TransformContext<'a>,
    el: &ElementNode<'a>,
    block: &mut BlockIRNode<'a>,
) {
    let element_id = ctx.next_id();

    match el.tag_type {
        ElementType::Element => {
            // Generate template string and register it
            let template = generate_element_template(el);
            ctx.add_template(element_id, template);

            // Process props and events
            for prop in el.props.iter() {
                match prop {
                    PropNode::Directive(dir) => {
                        transform_directive(ctx, dir, element_id, el, block);
                    }
                    PropNode::Attribute(_attr) => {
                        // Static attributes are included in the template
                    }
                }
            }

            // Check if we have mixed text and interpolation children
            let has_text_or_interpolation = el.children.iter().any(|c| {
                matches!(
                    c,
                    TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
                )
            });
            let has_interpolation = el
                .children
                .iter()
                .any(|c| matches!(c, TemplateChildNode::Interpolation(_)));

            if has_interpolation && has_text_or_interpolation {
                // Collect all text parts and interpolations together
                transform_text_children(ctx, &el.children, element_id, block);
            }

            // Process other dynamic children
            for child in el.children.iter() {
                match child {
                    TemplateChildNode::Interpolation(_) | TemplateChildNode::Text(_) => {
                        // Already handled above
                    }
                    TemplateChildNode::Element(child_el) => {
                        // Only process dynamic child elements
                        if !is_static_element(child_el) {
                            transform_element(ctx, child_el, block);
                        }
                    }
                    TemplateChildNode::If(if_node) => {
                        transform_if_node(ctx, if_node, block);
                    }
                    TemplateChildNode::For(for_node) => {
                        transform_for_node(ctx, for_node, block);
                    }
                    _ => {}
                }
            }
        }
        ElementType::Component => {
            // Component handling - process props and events
            let mut props = Vec::new_in(ctx.allocator);
            let slots = Vec::new_in(ctx.allocator);

            // Process props (v-bind and v-on directives, and static attributes)
            for prop in el.props.iter() {
                match prop {
                    PropNode::Directive(dir) => {
                        if dir.name.as_str() == "bind" {
                            // v-bind -> prop
                            if let Some(ref arg) = dir.arg {
                                if let ExpressionNode::Simple(key_exp) = arg {
                                    let key_node = SimpleExpressionNode::new(
                                        key_exp.content.clone(),
                                        key_exp.is_static,
                                        key_exp.loc.clone(),
                                    );
                                    let key = Box::new_in(key_node, ctx.allocator);

                                    let mut values = Vec::new_in(ctx.allocator);
                                    if let Some(ref exp) = dir.exp {
                                        if let ExpressionNode::Simple(val_exp) = exp {
                                            let val_node = SimpleExpressionNode::new(
                                                val_exp.content.clone(),
                                                val_exp.is_static,
                                                val_exp.loc.clone(),
                                            );
                                            values.push(Box::new_in(val_node, ctx.allocator));
                                        }
                                    }

                                    props.push(IRProp {
                                        key,
                                        values,
                                        is_component: true,
                                    });
                                }
                            }
                        } else if dir.name.as_str() == "on" {
                            // v-on -> onXxx prop
                            if let Some(ref arg) = dir.arg {
                                if let ExpressionNode::Simple(event_exp) = arg {
                                    // Convert event name to onXxx format
                                    let event_name = event_exp.content.as_str();
                                    let on_name = if event_name.is_empty() {
                                        String::from("on")
                                    } else {
                                        let mut s = String::from("on");
                                        let mut chars = event_name.chars();
                                        if let Some(c) = chars.next() {
                                            s.push(c.to_ascii_uppercase());
                                        }
                                        for c in chars {
                                            s.push(c);
                                        }
                                        s
                                    };

                                    let key_node = SimpleExpressionNode::new(
                                        on_name,
                                        true,
                                        event_exp.loc.clone(),
                                    );
                                    let key = Box::new_in(key_node, ctx.allocator);

                                    let mut values = Vec::new_in(ctx.allocator);
                                    if let Some(ref exp) = dir.exp {
                                        if let ExpressionNode::Simple(val_exp) = exp {
                                            let val_node = SimpleExpressionNode::new(
                                                val_exp.content.clone(),
                                                val_exp.is_static,
                                                val_exp.loc.clone(),
                                            );
                                            values.push(Box::new_in(val_node, ctx.allocator));
                                        }
                                    }

                                    props.push(IRProp {
                                        key,
                                        values,
                                        is_component: true,
                                    });
                                }
                            }
                        }
                    }
                    PropNode::Attribute(attr) => {
                        // Static attribute -> prop
                        let key_node = SimpleExpressionNode::new(
                            attr.name.clone(),
                            true,
                            SourceLocation::STUB,
                        );
                        let key = Box::new_in(key_node, ctx.allocator);

                        let mut values = Vec::new_in(ctx.allocator);
                        if let Some(ref value) = attr.value {
                            let val_node = SimpleExpressionNode::new(
                                value.content.clone(),
                                true,
                                SourceLocation::STUB,
                            );
                            values.push(Box::new_in(val_node, ctx.allocator));
                        }

                        props.push(IRProp {
                            key,
                            values,
                            is_component: true,
                        });
                    }
                }
            }

            let create_component = CreateComponentIRNode {
                id: element_id,
                tag: el.tag.clone(),
                props,
                slots,
                asset: true,
                once: false,
                dynamic_slots: false,
            };

            block
                .operation
                .push(OperationNode::CreateComponent(create_component));
        }
        ElementType::Slot => {
            // Slot outlet handling
            let name_exp = SimpleExpressionNode::new("default", true, SourceLocation::STUB);
            let slot_outlet = SlotOutletIRNode {
                id: element_id,
                name: Box::new_in(name_exp, ctx.allocator),
                props: Vec::new_in(ctx.allocator),
                fallback: None,
            };

            block.operation.push(OperationNode::SlotOutlet(slot_outlet));
        }
        ElementType::Template => {
            // Template element - process children directly
            for child in el.children.iter() {
                match child {
                    TemplateChildNode::Element(child_el) => {
                        transform_element(ctx, child_el, block);
                    }
                    TemplateChildNode::Text(text) => {
                        transform_text(ctx, text, block);
                    }
                    TemplateChildNode::Interpolation(interp) => {
                        transform_interpolation(ctx, interp, block);
                    }
                    _ => {}
                }
            }
        }
    }

    block.returns.push(element_id);
}

/// Generate element template string (recursively includes static children)
pub(crate) fn generate_element_template(el: &ElementNode<'_>) -> String {
    let mut template = cstr!("<{}", el.tag);

    // Add static attributes
    for prop in el.props.iter() {
        if let PropNode::Attribute(attr) = prop {
            if let Some(ref value) = attr.value {
                append!(template, " {}=\"{}\"", attr.name, value.content);
            } else {
                append!(template, " {}", attr.name);
            }
        }
    }

    if el.is_self_closing {
        template.push_str(" />");
    } else {
        template.push('>');

        // Check if there are any interpolations - if so, use a space placeholder
        let has_interpolation = el
            .children
            .iter()
            .any(|c| matches!(c, TemplateChildNode::Interpolation(_)));

        if has_interpolation {
            // Use single space as placeholder for interpolation text content
            template.push(' ');
        } else {
            // Recursively add static children (text and static elements)
            for child in el.children.iter() {
                match child {
                    TemplateChildNode::Text(text) => {
                        template.push_str(&escape_html_text(&text.content));
                    }
                    TemplateChildNode::Element(child_el) => {
                        // Include child elements in template
                        template.push_str(&generate_element_template(child_el));
                    }
                    _ => {
                        // Other dynamic content is handled elsewhere
                    }
                }
            }
        }

        append!(template, "</{}>", el.tag);
    }

    template
}

/// Escape HTML special characters in text content (vuejs/core #14310)
pub(crate) fn escape_html_text(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

/// Check if an element is static (no dynamic directives)
pub(crate) fn is_static_element(el: &ElementNode<'_>) -> bool {
    // Check if any prop is a directive (dynamic)
    for prop in el.props.iter() {
        if matches!(prop, PropNode::Directive(_)) {
            return false;
        }
    }

    // Check if any child is dynamic
    for child in el.children.iter() {
        match child {
            TemplateChildNode::Interpolation(_) => return false,
            TemplateChildNode::Element(child_el) => {
                if !is_static_element(child_el) {
                    return false;
                }
            }
            TemplateChildNode::If(_) | TemplateChildNode::For(_) => return false,
            _ => {}
        }
    }

    true
}
