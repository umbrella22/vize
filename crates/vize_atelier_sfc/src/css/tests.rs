//! Tests for CSS compilation.

use vize_carton::ToCompactString;
use vize_carton::{Bump, BumpVec};

use super::scoped::{
    add_scope_to_element, apply_scoped_css, transform_deep, transform_global, transform_slotted,
};
use super::transform::extract_and_transform_v_bind;
#[cfg(feature = "native")]
use super::CssTargets;
use super::{compile_css, CssCompileOptions};

#[test]
fn test_compile_simple_css() {
    let css = ".foo { color: red; }";
    let result = compile_css(css, &CssCompileOptions::default());
    assert!(result.errors.is_empty());
    assert!(result.code.contains(".foo"));
    assert!(result.code.contains("color"));
}

#[test]
fn test_compile_scoped_css() {
    let css = ".foo { color: red; }";
    let result = compile_css(
        css,
        &CssCompileOptions {
            scoped: true,
            scope_id: Some("data-v-123".to_compact_string()),
            ..Default::default()
        },
    );
    assert!(result.errors.is_empty());
    assert!(result.code.contains("[data-v-123]"));
}

#[test]
#[cfg(feature = "native")]
fn test_compile_minified_css() {
    let css = ".foo {\n  color: red;\n  background: blue;\n}";
    let result = compile_css(
        css,
        &CssCompileOptions {
            minify: true,
            ..Default::default()
        },
    );
    assert!(result.errors.is_empty());
    // Minified should have no newlines in simple case
    assert!(!result.code.contains('\n') || result.code.lines().count() == 1);
}

#[test]
fn test_v_bind_extraction() {
    let bump = Bump::new();
    let css = ".foo { color: v-bind(color); background: v-bind('bgColor'); }";
    let (transformed, vars) = extract_and_transform_v_bind(&bump, css);
    assert_eq!(vars.len(), 2);
    assert!(vars.contains(&"color".to_compact_string()));
    assert!(vars.contains(&"bgColor".to_compact_string()));
    assert!(transformed.contains("var(--"));
}

#[test]
fn test_scope_deep() {
    let bump = Bump::new();
    let mut out = BumpVec::new_in(&bump);
    transform_deep(&mut out, ":deep(.child)", 0, b"[data-v-123]");
    let result = unsafe { std::str::from_utf8_unchecked(&out) };
    assert_eq!(result, "[data-v-123] .child");
}

#[test]
fn test_scope_global() {
    let bump = Bump::new();
    let mut out = BumpVec::new_in(&bump);
    transform_global(&mut out, ":global(.foo)", 0);
    let result = unsafe { std::str::from_utf8_unchecked(&out) };
    assert_eq!(result, ".foo");
}

#[test]
fn test_scope_slotted() {
    let bump = Bump::new();
    let mut out = BumpVec::new_in(&bump);
    transform_slotted(&mut out, ":slotted(.child)", 0, b"[data-v-123]");
    let result = unsafe { std::str::from_utf8_unchecked(&out) };
    assert_eq!(result, ".child[data-v-123-s]");
}

#[test]
fn test_scope_slotted_with_pseudo() {
    let bump = Bump::new();
    let mut out = BumpVec::new_in(&bump);
    transform_slotted(&mut out, ":slotted(.child):hover", 0, b"[data-v-abc]");
    let result = unsafe { std::str::from_utf8_unchecked(&out) };
    assert_eq!(result, ".child[data-v-abc-s]:hover");
}

#[test]
fn test_scope_slotted_complex() {
    let bump = Bump::new();
    let mut out = BumpVec::new_in(&bump);
    transform_slotted(&mut out, ":slotted(div.foo)", 0, b"[data-v-12345678]");
    let result = unsafe { std::str::from_utf8_unchecked(&out) };
    assert_eq!(result, "div.foo[data-v-12345678-s]");
}

#[test]
fn test_scope_with_pseudo_element() {
    let bump = Bump::new();
    let mut out = BumpVec::new_in(&bump);
    add_scope_to_element(&mut out, ".foo::before", b"[data-v-123]");
    let result = unsafe { std::str::from_utf8_unchecked(&out) };
    assert_eq!(result, ".foo[data-v-123]::before");
}

#[test]
fn test_scope_with_pseudo_class() {
    let bump = Bump::new();
    let mut out = BumpVec::new_in(&bump);
    add_scope_to_element(&mut out, ".foo:hover", b"[data-v-123]");
    let result = unsafe { std::str::from_utf8_unchecked(&out) };
    assert_eq!(result, ".foo[data-v-123]:hover");
}

#[test]
#[cfg(feature = "native")]
fn test_compile_with_targets() {
    let css = ".foo { display: flex; }";
    let result = compile_css(
        css,
        &CssCompileOptions {
            targets: Some(CssTargets {
                chrome: Some(80),
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    assert!(result.errors.is_empty());
    assert!(result.code.contains("flex"));
}

#[test]
fn test_scoped_css_with_quoted_font_family() {
    let css = ".foo { font-family: 'JetBrains Mono', monospace; }";
    let result = compile_css(
        css,
        &CssCompileOptions {
            scoped: true,
            scope_id: Some("data-v-123".to_compact_string()),
            ..Default::default()
        },
    );
    println!("Result: {}", result.code);
    assert!(result.errors.is_empty());
    // Note: LightningCSS may remove quotes from font names
    assert!(
        result.code.contains("JetBrains Mono"),
        "Expected font name in: {}",
        result.code
    );
    assert!(result.code.contains("monospace"));
}

#[test]
fn test_apply_scoped_css_at_media() {
    let bump = Bump::new();
    // Root-level @media with selectors inside
    let css = ".foo { color: red; }\n@media (max-width: 768px) { .foo { color: blue; } }";
    let result = apply_scoped_css(&bump, css, "data-v-123");
    println!("@media result: {}", result);
    assert!(
        result.contains("@media (max-width: 768px)"),
        "Expected @media query preserved in: {}",
        result
    );
    // Both .foo selectors should be scoped
    assert_eq!(
        result.matches("[data-v-123]").count(),
        2,
        "Expected 2 scope attributes in: {}",
        result
    );
}

#[test]
fn test_apply_scoped_css_at_media_custom_media() {
    let bump = Bump::new();
    // @media with custom media queries (like --mobile)
    let css = ".a { color: red; }\n@media (--mobile) { .a { font-size: 12px; } }";
    let result = apply_scoped_css(&bump, css, "data-v-abc");
    println!("Custom media result: {}", result);
    assert!(
        result.contains("@media (--mobile)"),
        "Expected @media (--mobile) preserved in: {}",
        result
    );
    assert_eq!(
        result.matches("[data-v-abc]").count(),
        2,
        "Expected 2 scope attributes in: {}",
        result
    );
}

#[test]
fn test_apply_scoped_css_multiple_selectors_in_media() {
    let bump = Bump::new();
    // Multiple selectors inside @media
    let css = "@media (--mobile) { .a { color: red; } .b { color: blue; } }";
    let result = apply_scoped_css(&bump, css, "data-v-xyz");
    println!("Multi selector result: {}", result);
    assert!(result.contains("@media (--mobile)"));
    assert_eq!(
        result.matches("[data-v-xyz]").count(),
        2,
        "Expected 2 scope attributes in: {}",
        result
    );
}

#[test]
fn test_apply_scoped_css_with_quoted_string() {
    let bump = Bump::new();
    // Test the raw scoping function without LightningCSS
    let css = ".foo { font-family: 'JetBrains Mono', monospace; }";
    let result = apply_scoped_css(&bump, css, "data-v-123");
    println!("Scoped result: {}", result);
    assert!(
        result.contains("'JetBrains Mono'"),
        "Expected quoted font name in: {}",
        result
    );
    assert!(result.contains("monospace"));
}

#[test]
fn test_apply_scoped_css_at_import() {
    let bump = Bump::new();
    // @import should be preserved and not treated as a block at-rule
    let css = "@import \"~/assets/styles/custom-media-query.css\";\n\nfooter { width: 100%; }";
    let result = apply_scoped_css(&bump, css, "data-v-123");
    println!("@import result: {}", result);
    assert!(
        result.contains("@import \"~/assets/styles/custom-media-query.css\";"),
        "Expected @import preserved in: {}",
        result
    );
    assert!(
        result.contains("footer[data-v-123]"),
        "Expected footer scoped in: {}",
        result
    );
}

#[test]
fn test_apply_scoped_css_at_import_with_nested_css() {
    let bump = Bump::new();
    // @import followed by CSS nesting with @media
    let css = "@import \"custom.css\";\n\nfooter {\n  width: 100%;\n  @media (--mobile) {\n    padding: 1rem;\n  }\n}";
    let result = apply_scoped_css(&bump, css, "data-v-abc");
    println!("@import + nesting result: {}", result);
    assert!(
        result.contains("@import \"custom.css\";"),
        "Expected @import preserved in: {}",
        result
    );
    assert!(
        result.contains("footer[data-v-abc]"),
        "Expected footer scoped in: {}",
        result
    );
    // Nested @media should be inside the scoped footer block
    assert!(
        result.contains("@media (--mobile)"),
        "Expected nested @media preserved in: {}",
        result
    );
}
