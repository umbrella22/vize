//! Tests for template analysis.

use super::super::{Analyzer, AnalyzerOptions};

#[test]
fn test_vif_guard_in_template() {
    use vize_armature::parse;
    use vize_carton::Bump;

    let allocator = Bump::new();
    let template = r#"<div>
            <p v-if="todo.description">{{ unwrapDescription(todo.description) }}</p>
            <span>{{ todo.title }}</span>
        </div>"#;

    let (root, errors) = parse(&allocator, template);
    assert!(errors.is_empty(), "Template should parse without errors");

    let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
    analyzer.analyze_template(&root);
    let summary = analyzer.finish();

    // Find the interpolation expressions
    let expressions: Vec<_> = summary
        .template_expressions
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                crate::analysis::TemplateExpressionKind::Interpolation
            )
        })
        .collect();

    assert_eq!(expressions.len(), 2, "Should have 2 interpolations");

    // First interpolation is inside v-if, should have guard
    let inside_vif = expressions
        .iter()
        .find(|e| e.content.contains("unwrapDescription"))
        .expect("Should find unwrapDescription interpolation");
    assert!(
        inside_vif.vif_guard.is_some(),
        "Interpolation inside v-if should have vif_guard, got: {:?}",
        inside_vif.vif_guard
    );
    assert_eq!(
        inside_vif.vif_guard.as_deref(),
        Some("todo.description"),
        "vif_guard should be the v-if condition"
    );

    // Second interpolation is outside v-if, should NOT have guard
    let outside_vif = expressions
        .iter()
        .find(|e| e.content.contains("todo.title"))
        .expect("Should find todo.title interpolation");
    assert!(
        outside_vif.vif_guard.is_none(),
        "Interpolation outside v-if should NOT have vif_guard"
    );
}
