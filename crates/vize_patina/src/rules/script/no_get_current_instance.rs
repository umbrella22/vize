//! script/no-get-current-instance
//!
//! Disallow `getCurrentInstance()` in Vapor mode.
//!
//! In Vapor mode, `getCurrentInstance()` returns `null`. Code that relies
//! on this function will not work correctly in Vapor components.
//!
//! Based on Vue 3.6.0-beta.1 release notes:
//! <https://github.com/vuejs/core/releases/tag/v3.6.0-beta.1>
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! import { getCurrentInstance } from 'vue'
//!
//! const instance = getCurrentInstance()
//! const proxy = instance?.proxy
//! ```
//!
//! ### Valid
//! ```ts
//! // Use Composition API alternatives instead
//! import { useAttrs, useSlots } from 'vue'
//!
//! const attrs = useAttrs()
//! const slots = useSlots()
//! ```

use memchr::memmem;

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};
use crate::diagnostic::{LintDiagnostic, Severity};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-get-current-instance",
    description: "Disallow getCurrentInstance() in Vapor mode (returns null)",
    default_severity: Severity::Error,
};

/// Disallow getCurrentInstance()
pub struct NoGetCurrentInstance;

impl ScriptRule for NoGetCurrentInstance {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    #[inline]
    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();
        let finder = memmem::Finder::new(b"getCurrentInstance");
        let mut search_start = 0;

        while let Some(pos) = finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            let start = offset + abs_pos;
            let end = start + b"getCurrentInstance".len();

            result.add_diagnostic(
                LintDiagnostic::error(
                    META.name,
                    "getCurrentInstance() returns null in Vapor mode",
                    start as u32,
                    end as u32,
                )
                .with_help(
                    "getCurrentInstance() is not supported in Vapor components. \
                     Use Composition API alternatives like useAttrs(), useSlots(), \
                     or inject/provide for dependency injection.",
                ),
            );

            search_start = abs_pos + b"getCurrentInstance".len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NoGetCurrentInstance, ScriptLintResult, ScriptRule};

    #[test]
    fn test_valid_no_get_current_instance() {
        let source = r#"
import { ref, useAttrs } from 'vue'
const count = ref(0)
const attrs = useAttrs()
"#;
        let rule = NoGetCurrentInstance;
        let mut result = ScriptLintResult::default();
        rule.check(source, 0, &mut result);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_get_current_instance_import() {
        let source = r#"
import { getCurrentInstance } from 'vue'
const instance = getCurrentInstance()
"#;
        let rule = NoGetCurrentInstance;
        let mut result = ScriptLintResult::default();
        rule.check(source, 0, &mut result);
        // Should find 2: one in import, one in usage
        assert_eq!(result.error_count, 2);
    }

    #[test]
    fn test_invalid_get_current_instance_usage() {
        let source = "const proxy = getCurrentInstance()?.proxy";
        let rule = NoGetCurrentInstance;
        let mut result = ScriptLintResult::default();
        rule.check(source, 0, &mut result);
        assert_eq!(result.error_count, 1);
        assert!(result.diagnostics[0].message.contains("getCurrentInstance"));
    }
}
