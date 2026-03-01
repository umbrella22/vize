//! vue/no-multi-spaces
//!
//! Disallow multiple consecutive spaces in template.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div  class="foo"></div>
//! <div class="foo"  id="bar"></div>
//! ```
//!
//! ### Valid
//! ```vue
//! <div class="foo"></div>
//! <div class="foo" id="bar"></div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::{Fix, LintDiagnostic, Severity, TextEdit};
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::ElementNode;

static META: RuleMeta = RuleMeta {
    name: "vue/no-multi-spaces",
    description: "Disallow multiple consecutive spaces",
    category: RuleCategory::StronglyRecommended,
    fixable: true,
    default_severity: Severity::Warning,
};

/// Disallow multiple spaces
pub struct NoMultiSpaces {
    /// Ignore properties (v-if, v-for expressions)
    pub ignore_properties: bool,
}

impl Default for NoMultiSpaces {
    fn default() -> Self {
        Self {
            ignore_properties: true,
        }
    }
}

impl Rule for NoMultiSpaces {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        // Check spacing between attributes
        let props: Vec<_> = element.props.iter().collect();

        for i in 0..props.len() {
            if i > 0 {
                let prev_end = props[i - 1].loc().end.offset;
                let curr_start = props[i].loc().start.offset;

                // Note: end.offset is inclusive (points AT the last char, not after it)
                // So the gap = curr_start - prev_end - 1 represents actual whitespace
                // Example: prev_end=15 (quote), curr_start=17 (i in id) -> gap = 1 space at pos 16
                if curr_start > prev_end + 2 {
                    // More than one space between attributes
                    let fix = Fix::new(
                        "Replace multiple spaces with single space",
                        TextEdit::replace(prev_end + 1, curr_start, " "),
                    );

                    ctx.report(
                        LintDiagnostic::warn(
                            META.name,
                            "Multiple consecutive spaces",
                            prev_end + 1,
                            curr_start,
                        )
                        .with_fix(fix),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NoMultiSpaces;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(NoMultiSpaces::default()));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_single_space() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="foo" id="bar"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_multiple_spaces() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="foo"  id="bar"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].has_fix());
    }
}
