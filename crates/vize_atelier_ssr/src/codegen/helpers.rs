//! HTML escaping utilities and child/control-flow processing for SSR codegen.

use vize_atelier_core::ast::{
    CommentNode, ForNode, IfNode, InterpolationNode, RuntimeHelper, TemplateChildNode, TextNode,
};

use super::SsrCodegenContext;
use vize_carton::{cstr, String};

impl<'a> SsrCodegenContext<'a> {
    /// Process a list of children nodes
    pub(crate) fn process_children(
        &mut self,
        children: &[TemplateChildNode],
        as_fragment: bool,
        disable_nested_fragments: bool,
        disable_comment: bool,
    ) {
        if as_fragment {
            self.push_string_part_static("<!--[-->");
        }

        for child in children {
            self.process_child(child, disable_nested_fragments, disable_comment);
        }

        if as_fragment {
            self.push_string_part_static("<!--]-->");
        }
    }

    /// Process a single child node
    fn process_child(
        &mut self,
        child: &TemplateChildNode,
        disable_nested_fragments: bool,
        disable_comment: bool,
    ) {
        match child {
            TemplateChildNode::Element(el) => {
                self.process_element(el, disable_nested_fragments);
            }
            TemplateChildNode::Text(text) => {
                self.process_text(text);
            }
            TemplateChildNode::Comment(comment) => {
                if !disable_comment {
                    self.process_comment(comment);
                }
            }
            TemplateChildNode::Interpolation(interp) => {
                self.process_interpolation(interp);
            }
            TemplateChildNode::If(if_node) => {
                self.process_if(if_node, disable_nested_fragments, disable_comment);
            }
            TemplateChildNode::For(for_node) => {
                self.process_for(for_node, disable_nested_fragments);
            }
            TemplateChildNode::IfBranch(_) => {
                // Handled by process_if
            }
            TemplateChildNode::TextCall(_) | TemplateChildNode::CompoundExpression(_) => {
                // These don't appear in SSR since transformText is not used
            }
            TemplateChildNode::Hoisted(_) => {
                // Hoisting is not used in SSR
            }
        }
    }

    /// Process a text node
    fn process_text(&mut self, text: &TextNode) {
        self.push_string_part_static(&escape_html(&text.content));
    }

    /// Process a comment node
    fn process_comment(&mut self, comment: &CommentNode) {
        self.push_string_part_static("<!--");
        self.push_string_part_static(&comment.content);
        self.push_string_part_static("-->");
    }

    /// Process an interpolation node ({{ expr }})
    fn process_interpolation(&mut self, interp: &InterpolationNode) {
        use vize_atelier_core::ast::ExpressionNode;

        self.use_ssr_helper(RuntimeHelper::SsrInterpolate);

        let exp = match &interp.content {
            ExpressionNode::Simple(simple) => simple.content.as_str(),
            ExpressionNode::Compound(_) => "_ctx.value", // placeholder
        };

        self.push_string_part_dynamic(&cstr!("_ssrInterpolate({exp})"));
    }

    /// Process an if node
    pub(crate) fn process_if(
        &mut self,
        if_node: &IfNode,
        disable_nested_fragments: bool,
        disable_comment: bool,
    ) {
        // Flush current push before if statement
        self.flush_push();

        for (i, branch) in if_node.branches.iter().enumerate() {
            self.push_indent();

            if i == 0 {
                // First branch: if
                self.push("if (");
                if let Some(condition) = &branch.condition {
                    self.push_expression(condition);
                }
                self.push(") {\n");
            } else if branch.condition.is_some() {
                // else-if
                self.push("} else if (");
                if let Some(condition) = &branch.condition {
                    self.push_expression(condition);
                }
                self.push(") {\n");
            } else {
                // else
                self.push("} else {\n");
            }

            self.indent_level += 1;

            // Check if branch needs fragment
            let needs_fragment = !disable_nested_fragments && branch.children.len() > 1;

            self.process_children(
                &branch.children,
                needs_fragment,
                disable_nested_fragments,
                disable_comment,
            );
            self.flush_push();

            self.indent_level -= 1;
        }

        // If no else branch, emit empty comment
        if if_node.branches.iter().all(|b| b.condition.is_some()) {
            self.push_indent();
            self.push("} else {\n");
            self.indent_level += 1;
            self.push_string_part_static("<!---->");
            self.flush_push();
            self.indent_level -= 1;
        }

        self.push_indent();
        self.push("}\n");
    }

    /// Process a for node
    pub(crate) fn process_for(&mut self, for_node: &ForNode, disable_nested_fragments: bool) {
        // Flush current push before for statement
        self.flush_push();

        self.use_ssr_helper(RuntimeHelper::SsrRenderList);

        // Fragment markers for v-for
        if !disable_nested_fragments {
            self.push_indent();
            self.push("_push(`<!--[-->`)\n");
        }

        self.push_indent();
        self.push("_ssrRenderList(");
        self.push_expression(&for_node.source);
        self.push(", (");

        // Value alias
        if let Some(value) = &for_node.value_alias {
            self.push_expression(value);
        }
        // Key alias
        if let Some(key) = &for_node.key_alias {
            self.push(", ");
            self.push_expression(key);
        }
        // Index alias
        if let Some(index) = &for_node.object_index_alias {
            self.push(", ");
            self.push_expression(index);
        }

        self.push(") => {\n");
        self.indent_level += 1;

        // Process for body
        let needs_fragment = !disable_nested_fragments && for_node.children.len() > 1;
        self.process_children(&for_node.children, needs_fragment, true, false);
        self.flush_push();

        self.indent_level -= 1;
        self.push_indent();
        self.push("})\n");

        // Closing fragment marker
        if !disable_nested_fragments {
            self.push_indent();
            self.push("_push(`<!--]-->`)\n");
        }
    }

    /// Push an expression node
    pub(crate) fn push_expression(&mut self, expr: &vize_atelier_core::ast::ExpressionNode) {
        use vize_atelier_core::ast::ExpressionNode;

        match expr {
            ExpressionNode::Simple(simple) => {
                self.push(&simple.content);
            }
            ExpressionNode::Compound(compound) => {
                // Flatten compound expression
                for child in &compound.children {
                    use vize_atelier_core::ast::CompoundExpressionChild;
                    match child {
                        CompoundExpressionChild::Simple(s) => self.push(&s.content),
                        CompoundExpressionChild::String(s) => self.push(s),
                        CompoundExpressionChild::Symbol(helper) => {
                            self.push("_");
                            self.push(helper.name());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Escape HTML special characters
pub(crate) fn escape_html(s: &str) -> String {
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

/// Escape HTML attribute value
pub(crate) fn escape_html_attr(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
    }
    result
}
