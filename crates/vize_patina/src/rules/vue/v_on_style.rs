//! vue/v-on-style
//!
//! Enforce `v-on` directive style.
//!
//! ## Options
//!
//! - `"shorthand"` (default): Prefer `@event` over `v-on:event`
//! - `"longform"`: Prefer `v-on:event` over `@event`
//!
//! ## Examples
//!
//! ### Invalid (with shorthand option)
//! ```vue
//! <div v-on:click="handleClick"></div>
//! ```
//!
//! ### Valid (with shorthand option)
//! ```vue
//! <div @click="handleClick"></div>
//! ```

#![allow(clippy::disallowed_macros)]

use crate::context::LintContext;
use crate::diagnostic::{Fix, LintDiagnostic, Severity, TextEdit};
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_carton::String;
use vize_carton::ToCompactString;
use vize_relief::ast::{DirectiveNode, ElementNode};

static META: RuleMeta = RuleMeta {
    name: "vue/v-on-style",
    description: "Enforce `v-on` directive style",
    category: RuleCategory::StronglyRecommended,
    fixable: true,
    default_severity: Severity::Warning,
};

/// Style preference for v-on
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VOnStyleOption {
    #[default]
    Shorthand,
    Longform,
}

/// Enforce v-on directive style
pub struct VOnStyle {
    pub style: VOnStyleOption,
}

impl Default for VOnStyle {
    fn default() -> Self {
        Self {
            style: VOnStyleOption::Shorthand,
        }
    }
}

impl Rule for VOnStyle {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_directive<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        _element: &ElementNode<'a>,
        directive: &DirectiveNode<'a>,
    ) {
        if directive.name.as_str() != "on" {
            return;
        }

        // Skip object binding syntax (v-on="...")
        if directive.arg.is_none() {
            return;
        }

        let raw_name = directive.raw_name.as_deref().unwrap_or("");
        let is_shorthand = raw_name.starts_with('@');

        match self.style {
            VOnStyleOption::Shorthand => {
                if !is_shorthand {
                    let arg_content = directive
                        .arg
                        .as_ref()
                        .map(|a| match a {
                            vize_relief::ast::ExpressionNode::Simple(s) => {
                                s.content.to_compact_string()
                            }
                            _ => String::default(),
                        })
                        .unwrap_or_default();

                    let exp_content = directive.exp.as_ref().map(|e| match e {
                        vize_relief::ast::ExpressionNode::Simple(s) => {
                            s.content.to_compact_string()
                        }
                        _ => String::default(),
                    });

                    let modifiers = directive
                        .modifiers
                        .iter()
                        .map(|m| format!(".{}", m.content))
                        .collect::<String>();

                    let new_text = if let Some(exp) = exp_content {
                        format!("@{}{}=\"{}\"", arg_content, modifiers, exp)
                    } else {
                        format!("@{}{}", arg_content, modifiers)
                    };

                    let fix = Fix::new(
                        "Use shorthand syntax",
                        TextEdit::replace(
                            directive.loc.start.offset,
                            directive.loc.end.offset,
                            new_text,
                        ),
                    );

                    ctx.report(
                        LintDiagnostic::warn(
                            META.name,
                            "Prefer shorthand `@` over `v-on:`",
                            directive.loc.start.offset,
                            directive.loc.end.offset,
                        )
                        .with_help("Use `@event=\"handler\"` instead of `v-on:event=\"handler\"`")
                        .with_fix(fix),
                    );
                }
            }
            VOnStyleOption::Longform => {
                if is_shorthand {
                    let arg_content = directive
                        .arg
                        .as_ref()
                        .map(|a| match a {
                            vize_relief::ast::ExpressionNode::Simple(s) => {
                                s.content.to_compact_string()
                            }
                            _ => String::default(),
                        })
                        .unwrap_or_default();

                    let exp_content = directive.exp.as_ref().map(|e| match e {
                        vize_relief::ast::ExpressionNode::Simple(s) => {
                            s.content.to_compact_string()
                        }
                        _ => String::default(),
                    });

                    let modifiers = directive
                        .modifiers
                        .iter()
                        .map(|m| format!(".{}", m.content))
                        .collect::<String>();

                    let new_text = if let Some(exp) = exp_content {
                        format!("v-on:{}{}=\"{}\"", arg_content, modifiers, exp)
                    } else {
                        format!("v-on:{}{}", arg_content, modifiers)
                    };

                    let fix = Fix::new(
                        "Use longform syntax",
                        TextEdit::replace(
                            directive.loc.start.offset,
                            directive.loc.end.offset,
                            new_text,
                        ),
                    );

                    ctx.report(
                        LintDiagnostic::warn(
                            META.name,
                            "Prefer `v-on:` over shorthand `@`",
                            directive.loc.start.offset,
                            directive.loc.end.offset,
                        )
                        .with_help("Use `v-on:event=\"handler\"` instead of `@event=\"handler\"`")
                        .with_fix(fix),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VOnStyle;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter_shorthand() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(VOnStyle::default()));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_shorthand() {
        let linter = create_linter_shorthand();
        let result = linter.lint_template(r#"<div @click="handleClick"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_longform_with_shorthand_option() {
        let linter = create_linter_shorthand();
        let result = linter.lint_template(r#"<div v-on:click="handleClick"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].has_fix());
    }
}
