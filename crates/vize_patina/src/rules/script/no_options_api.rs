//! script/no-options-api
//!
//! Disallow Options API patterns in Vapor mode.
//!
//! Vue Vapor mode only supports Composition API. Options API patterns like
//! `data()`, `computed`, `methods`, `watch` as object properties are not
//! supported.
//!
//! Based on Vue 3.6.0-beta.1 release notes:
//! <https://github.com/vuejs/core/releases/tag/v3.6.0-beta.1>
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! export default {
//!   data() { return { count: 0 } },
//!   computed: { doubled() { return this.count * 2 } },
//!   methods: { increment() { this.count++ } },
//!   watch: { count(val) { console.log(val) } }
//! }
//! ```
//!
//! ### Valid
//! ```ts
//! import { ref, computed, watch } from 'vue'
//! const count = ref(0)
//! const doubled = computed(() => count.value * 2)
//! const increment = () => count.value++
//! watch(count, (val) => console.log(val))
//! ```

#![allow(clippy::disallowed_macros)]

use memchr::memmem;

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};
use crate::diagnostic::{LintDiagnostic, Severity};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-options-api",
    description: "Disallow Options API patterns in Vapor mode",
    default_severity: Severity::Error,
};

/// Options API patterns that indicate non-Vapor compatible code
const OPTIONS_API_PATTERNS: &[(&[u8], &str)] = &[
    (b"data()", "data() method"),
    (b"data ()", "data() method"),
    (b"computed:", "computed property"),
    (b"computed :", "computed property"),
    (b"methods:", "methods property"),
    (b"methods :", "methods property"),
    (b"watch:", "watch property (use watch() function)"),
    (b"watch :", "watch property (use watch() function)"),
    (b"props:", "props property (use defineProps())"),
    (b"props :", "props property (use defineProps())"),
    (b"emits:", "emits property (use defineEmits())"),
    (b"emits :", "emits property (use defineEmits())"),
    (b"setup()", "setup() method (use <script setup>)"),
    (b"setup ()", "setup() method (use <script setup>)"),
    (b"mounted()", "mounted() lifecycle hook"),
    (b"mounted ()", "mounted() lifecycle hook"),
    (b"created()", "created() lifecycle hook"),
    (b"created ()", "created() lifecycle hook"),
    (b"beforeMount()", "beforeMount() lifecycle hook"),
    (b"beforeMount ()", "beforeMount() lifecycle hook"),
    (b"beforeCreate()", "beforeCreate() lifecycle hook"),
    (b"beforeCreate ()", "beforeCreate() lifecycle hook"),
    (b"updated()", "updated() lifecycle hook"),
    (b"updated ()", "updated() lifecycle hook"),
    (b"beforeUpdate()", "beforeUpdate() lifecycle hook"),
    (b"beforeUpdate ()", "beforeUpdate() lifecycle hook"),
    (b"unmounted()", "unmounted() lifecycle hook"),
    (b"unmounted ()", "unmounted() lifecycle hook"),
    (b"beforeUnmount()", "beforeUnmount() lifecycle hook"),
    (b"beforeUnmount ()", "beforeUnmount() lifecycle hook"),
];

/// Disallow Options API patterns
pub struct NoOptionsApi;

impl ScriptRule for NoOptionsApi {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    #[inline]
    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Early bailout: check for "export default" which typically indicates Options API
        if memmem::find(bytes, b"export default").is_none() {
            return;
        }

        for (pattern, description) in OPTIONS_API_PATTERNS {
            if let Some(pos) = memmem::find(bytes, pattern) {
                let start = offset + pos;
                let end = start + pattern.len();

                result.add_diagnostic(
                    LintDiagnostic::error(
                        META.name,
                        format!(
                            "Options API '{}' is not supported in Vapor mode",
                            description
                        ),
                        start as u32,
                        end as u32,
                    )
                    .with_help(
                        "Vapor mode only supports Composition API. \
                         Use <script setup vapor> with Composition API functions.",
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NoOptionsApi, ScriptLintResult, ScriptRule};

    #[test]
    fn test_valid_composition_api() {
        let source = r#"
import { ref, computed } from 'vue'
const count = ref(0)
const doubled = computed(() => count.value * 2)
"#;
        let rule = NoOptionsApi;
        let mut result = ScriptLintResult::default();
        rule.check(source, 0, &mut result);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_data_option() {
        let source = r#"
export default {
  data() {
    return { count: 0 }
  }
}
"#;
        let rule = NoOptionsApi;
        let mut result = ScriptLintResult::default();
        rule.check(source, 0, &mut result);
        assert_eq!(result.error_count, 1);
        assert!(result.diagnostics[0].message.contains("data()"));
    }

    #[test]
    fn test_invalid_computed_option() {
        let source = r#"
export default {
  computed: {
    doubled() { return this.count * 2 }
  }
}
"#;
        let rule = NoOptionsApi;
        let mut result = ScriptLintResult::default();
        rule.check(source, 0, &mut result);
        assert_eq!(result.error_count, 1);
        assert!(result.diagnostics[0].message.contains("computed"));
    }

    #[test]
    fn test_invalid_methods_option() {
        let source = r#"
export default {
  methods: {
    increment() { this.count++ }
  }
}
"#;
        let rule = NoOptionsApi;
        let mut result = ScriptLintResult::default();
        rule.check(source, 0, &mut result);
        assert_eq!(result.error_count, 1);
        assert!(result.diagnostics[0].message.contains("methods"));
    }

    #[test]
    fn test_no_export_default_skip() {
        let source = r#"
const computed = { foo: 'bar' }
"#;
        let rule = NoOptionsApi;
        let mut result = ScriptLintResult::default();
        rule.check(source, 0, &mut result);
        assert_eq!(result.error_count, 0);
    }
}
