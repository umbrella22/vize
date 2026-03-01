//! Props generation functions.

mod directives;
mod events;
mod generate;

use crate::ast::{ExpressionNode, PropNode, RuntimeHelper};

use super::{context::CodegenContext, expression::generate_expression};

pub use directives::{generate_directive_prop_with_static, is_supported_directive};
pub use generate::generate_props;

/// Check if there's a v-bind without argument (object spread)
pub(crate) fn has_vbind_object(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            return dir.name == "bind" && dir.arg.is_none();
        }
        false
    })
}

/// Check if there's a v-on without argument (event object spread)
pub(crate) fn has_von_object(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            return dir.name == "on" && dir.arg.is_none();
        }
        false
    })
}

/// Check if there are other props besides v-bind/v-on object spreads
fn has_other_props(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| match p {
        PropNode::Attribute(_) => true,
        PropNode::Directive(dir) => {
            // v-bind without arg is the object spread, not a regular prop
            if dir.name == "bind" && dir.arg.is_none() {
                return false;
            }
            // v-on without arg is the event object spread, not a regular prop
            if dir.name == "on" && dir.arg.is_none() {
                return false;
            }
            is_supported_directive(dir)
        }
    })
}

/// Generate the v-bind object expression
pub(crate) fn generate_vbind_object_exp(ctx: &mut CodegenContext, props: &[PropNode<'_>]) {
    for p in props {
        if let PropNode::Directive(dir) = p {
            if dir.name == "bind" && dir.arg.is_none() {
                if let Some(exp) = &dir.exp {
                    generate_expression(ctx, exp);
                    return;
                }
            }
        }
    }
}

/// Generate the v-on object expression wrapped with toHandlers
pub(crate) fn generate_von_object_exp(ctx: &mut CodegenContext, props: &[PropNode<'_>]) {
    ctx.use_helper(RuntimeHelper::ToHandlers);
    ctx.push(ctx.helper(RuntimeHelper::ToHandlers));
    ctx.push("(");
    for p in props {
        if let PropNode::Directive(dir) = p {
            if dir.name == "on" && dir.arg.is_none() {
                if let Some(exp) = &dir.exp {
                    generate_expression(ctx, exp);
                    ctx.push(", true"); // true for handlerOnly
                    break;
                }
            }
        }
    }
    ctx.push(")");
}

/// Check if any v-bind prop has a dynamic key (v-bind with dynamic arg)
/// Note: v-on with dynamic arg uses _toHandlerKey() instead and doesn't need _normalizeProps
fn has_dynamic_key(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                    return !exp.is_static;
                }
            }
        }
        false
    })
}

/// Check if element has dynamic v-model (with dynamic argument)
pub fn has_dynamic_vmodel(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "model" {
                return dir.arg.as_ref().is_some_and(|arg| match arg {
                    ExpressionNode::Simple(exp) => !exp.is_static,
                    ExpressionNode::Compound(_) => true,
                });
            }
        }
        false
    })
}
