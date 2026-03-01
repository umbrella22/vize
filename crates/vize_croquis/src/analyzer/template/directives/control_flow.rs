//! Control flow directive handling.
//!
//! Processes v-if/v-else-if/v-else and v-for nodes at the template AST level
//! (as opposed to directive-level processing in `visit_element`).

use crate::analyzer::Analyzer;
use crate::scope::VForScopeData;
use crate::ScopeBinding;
use vize_carton::CompactString;
use vize_relief::ast::{ExpressionNode, ForNode, IfNode, PropNode};
use vize_relief::BindingType;

impl Analyzer {
    /// Visit if node.
    pub(in crate::analyzer) fn visit_if(
        &mut self,
        if_node: &IfNode<'_>,
        scope_vars: &mut Vec<CompactString>,
    ) {
        for branch in if_node.branches.iter() {
            if self.options.detect_undefined && self.script_analyzed {
                if let Some(ref cond) = branch.condition {
                    self.check_expression_refs(cond, scope_vars, branch.loc.start.offset);
                }
            }

            if self.options.detect_undefined && self.script_analyzed {
                if let Some(PropNode::Directive(dir)) = &branch.user_key {
                    if let Some(ref exp) = dir.exp {
                        self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                    }
                }
            }

            // Push v-if guard for type narrowing
            let guard_pushed = if let Some(ref cond) = branch.condition {
                let cond_str = match cond {
                    ExpressionNode::Simple(s) => s.content.as_str(),
                    ExpressionNode::Compound(c) => c.loc.source.as_str(),
                };
                self.vif_guard_stack.push(CompactString::new(cond_str));
                true
            } else {
                false
            };

            for child in branch.children.iter() {
                self.visit_template_child(child, scope_vars);
            }

            // Pop v-if guard
            if guard_pushed {
                self.vif_guard_stack.pop();
            }
        }
    }

    /// Visit for node.
    pub(in crate::analyzer) fn visit_for(
        &mut self,
        for_node: &ForNode<'_>,
        scope_vars: &mut Vec<CompactString>,
    ) {
        let vars_added = self.extract_for_vars(for_node);
        let vars_count = vars_added.len();

        if self.options.analyze_template_scopes && !vars_added.is_empty() {
            let source_content = match &for_node.source {
                ExpressionNode::Simple(s) => CompactString::new(s.content.as_str()),
                ExpressionNode::Compound(c) => CompactString::new(c.loc.source.as_str()),
            };

            let value_alias = vars_added
                .first()
                .cloned()
                .unwrap_or_else(|| CompactString::const_new("_"));

            self.summary.scopes.enter_v_for_scope(
                VForScopeData {
                    value_alias,
                    key_alias: vars_added.get(1).cloned(),
                    index_alias: vars_added.get(2).cloned(),
                    source: source_content,
                    key_expression: None,
                },
                for_node.loc.start.offset,
                for_node.loc.end.offset,
            );
            for var in &vars_added {
                self.summary
                    .scopes
                    .add_binding(var.clone(), ScopeBinding::new(BindingType::SetupConst, 0));
            }
        }

        for var in vars_added {
            scope_vars.push(var);
        }

        if self.options.detect_undefined && self.script_analyzed {
            self.check_expression_refs(&for_node.source, scope_vars, for_node.loc.start.offset);
        }

        for child in for_node.children.iter() {
            self.visit_template_child(child, scope_vars);
        }

        for _ in 0..vars_count {
            scope_vars.pop();
        }
        if self.options.analyze_template_scopes && vars_count > 0 {
            self.summary.scopes.exit_scope();
        }
    }

    /// Extract variables from v-for expression.
    fn extract_for_vars(&self, for_node: &ForNode<'_>) -> Vec<CompactString> {
        let mut vars = Vec::new();

        if let Some(ExpressionNode::Simple(exp)) = &for_node.value_alias {
            vars.push(exp.content.clone());
        }

        if let Some(ExpressionNode::Simple(exp)) = &for_node.key_alias {
            vars.push(exp.content.clone());
        }

        if let Some(ExpressionNode::Simple(exp)) = &for_node.object_index_alias {
            vars.push(exp.content.clone());
        }

        vars
    }
}
