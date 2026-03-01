//! tsgo Bridge - Communication layer with TypeScript type checker.
//!
//! This module provides a bridge to tsgo (TypeScript Go implementation)
//! via LSP protocol over stdio. It enables in-memory type checking
//! without writing temporary files to disk.

mod bridge;
mod protocol;
mod reader;
mod types;

pub use bridge::{BatchTypeChecker, TsgoBridge};
pub use types::{
    LspCompletionItem, LspCompletionList, LspCompletionResponse, LspDefinitionResponse,
    LspDiagnostic, LspDocumentation, LspHover, LspHoverContents, LspLocation, LspLocationLink,
    LspMarkedString, LspMarkupContent, LspPosition, LspRange, TsgoBridgeConfig, TsgoBridgeError,
    TypeCheckResult, VIRTUAL_URI_SCHEME,
};

#[cfg(test)]
mod tests {
    use super::{
        LspDiagnostic, LspPosition, LspRange, TsgoBridgeConfig, TypeCheckResult, VIRTUAL_URI_SCHEME,
    };
    use vize_carton::cstr;

    #[test]
    fn test_virtual_uri_format() {
        let name = "Component.vue.ts";
        let uri = cstr!("{VIRTUAL_URI_SCHEME}://{name}");
        assert_eq!(uri, "vize-virtual://Component.vue.ts");
    }

    #[test]
    fn test_type_check_result() {
        let mut result = TypeCheckResult::default();
        assert!(!result.has_errors());
        assert_eq!(result.error_count(), 0);

        result.diagnostics.push(LspDiagnostic {
            range: LspRange {
                start: LspPosition {
                    line: 0,
                    character: 0,
                },
                end: LspPosition {
                    line: 0,
                    character: 10,
                },
            },
            severity: Some(1),
            code: None,
            source: Some("ts".into()),
            message: "Type error".into(),
            related_information: None,
        });

        assert!(result.has_errors());
        assert_eq!(result.error_count(), 1);
        assert_eq!(result.warning_count(), 0);
    }

    #[test]
    fn test_config_default() {
        let config = TsgoBridgeConfig::default();
        assert!(config.tsgo_path.is_none());
        assert!(config.working_dir.is_none());
        assert_eq!(config.timeout_ms, 30000);
        assert!(!config.enable_profiling);
    }
}
