//! SSR compiler options.

use serde::{Deserialize, Serialize};
use vize_carton::String;

/// SSR compiler options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsrCompilerOptions {
    /// Scope ID for scoped CSS (data-v-xxx)
    #[serde(default)]
    pub scope_id: Option<String>,

    /// Whether to preserve comments
    #[serde(default)]
    pub comments: bool,

    /// Whether to inline template
    #[serde(default)]
    pub inline: bool,

    /// Whether is TypeScript
    #[serde(default)]
    pub is_ts: bool,

    /// CSS variables to inject (from SFC <style> blocks with v-bind)
    #[serde(default)]
    pub ssr_css_vars: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::SsrCompilerOptions;

    #[test]
    fn test_default_options() {
        let opts = SsrCompilerOptions::default();
        assert!(opts.scope_id.is_none());
        assert!(!opts.comments);
        assert!(!opts.inline);
        assert!(!opts.is_ts);
        assert!(opts.ssr_css_vars.is_none());
    }
}
