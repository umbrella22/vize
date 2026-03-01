//! v-for directive transform.
//!
//! Transforms elements with v-for directive into ForNode.

use vize_carton::{Box, Bump};

use crate::ast::*;
use crate::transform::TransformContext;

/// Check if an element has a v-for directive
pub fn has_v_for(el: &ElementNode<'_>) -> bool {
    el.props
        .iter()
        .any(|prop| matches!(prop, PropNode::Directive(dir) if dir.name == "for"))
}

/// Get the v-for expression from an element
pub fn get_for_expression<'a>(el: &'a ElementNode<'a>) -> Option<&'a ExpressionNode<'a>> {
    for prop in el.props.iter() {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "for" {
                return dir.exp.as_ref();
            }
        }
    }
    None
}

/// Remove v-for directive from element props
pub fn remove_for_directive(el: &mut ElementNode<'_>) {
    let mut i = 0;
    while i < el.props.len() {
        if let PropNode::Directive(dir) = &el.props[i] {
            if dir.name == "for" {
                el.props.remove(i);
                return;
            }
        }
        i += 1;
    }
}

/// Parse v-for expression into parts
pub fn parse_for_expression<'a>(
    allocator: &'a Bump,
    content: &str,
    loc: &SourceLocation,
) -> ForParseResult<'a> {
    // Match patterns like "item in items" or "(item, index) in items"
    let (alias_part, source_part) = if let Some(idx) = content.find(" in ") {
        (&content[..idx], &content[idx + 4..])
    } else if let Some(idx) = content.find(" of ") {
        (&content[..idx], &content[idx + 4..])
    } else {
        let source = ExpressionNode::Simple(Box::new_in(
            SimpleExpressionNode::new(content, false, loc.clone()),
            allocator,
        ));
        return ForParseResult {
            source,
            value: None,
            key: None,
            index: None,
            finalized: false,
        };
    };

    let source_str = source_part.trim();
    let alias_str = alias_part.trim();

    let source = ExpressionNode::Simple(Box::new_in(
        SimpleExpressionNode::new(source_str, false, SourceLocation::default()),
        allocator,
    ));

    let (value, key, index) = if alias_str.starts_with('(') && alias_str.ends_with(')') {
        let inner = &alias_str[1..alias_str.len() - 1];
        let aliases: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

        let value = if !aliases.is_empty() && !aliases[0].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode::new(aliases[0], false, SourceLocation::default()),
                allocator,
            )))
        } else {
            None
        };

        let key = if aliases.len() > 1 && !aliases[1].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode::new(aliases[1], false, SourceLocation::default()),
                allocator,
            )))
        } else {
            None
        };

        let index = if aliases.len() > 2 && !aliases[2].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode::new(aliases[2], false, SourceLocation::default()),
                allocator,
            )))
        } else {
            None
        };

        (value, key, index)
    } else {
        let value = Some(ExpressionNode::Simple(Box::new_in(
            SimpleExpressionNode::new(alias_str, false, SourceLocation::default()),
            allocator,
        )));
        (value, None, None)
    };

    ForParseResult {
        source,
        value,
        key,
        index,
        finalized: false,
    }
}

/// Process v-for structural directive - adds helpers
pub fn process_v_for(ctx: &mut TransformContext<'_>) {
    ctx.helper(RuntimeHelper::RenderList);
    ctx.helper(RuntimeHelper::OpenBlock);
    ctx.helper(RuntimeHelper::CreateBlock);
    ctx.helper(RuntimeHelper::Fragment);
}

#[cfg(test)]
mod tests {
    use super::{
        has_v_for, parse_for_expression, ExpressionNode, SourceLocation, TemplateChildNode,
    };
    use crate::parser::parse;
    use bumpalo::Bump;

    #[test]
    fn test_has_v_for() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, r#"<div v-for="item in items">{{ item }}</div>"#);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert!(has_v_for(el));
        }
    }

    #[test]
    fn test_parse_simple_for() {
        let allocator = Bump::new();
        let result = parse_for_expression(&allocator, "item in items", &SourceLocation::STUB);

        if let ExpressionNode::Simple(source) = &result.source {
            assert_eq!(source.content.as_str(), "items");
        }
        assert!(result.value.is_some());
    }

    #[test]
    fn test_parse_for_with_index() {
        let allocator = Bump::new();
        let result =
            parse_for_expression(&allocator, "(item, index) in items", &SourceLocation::STUB);

        if let ExpressionNode::Simple(source) = &result.source {
            assert_eq!(source.content.as_str(), "items");
        }
        assert!(result.value.is_some());
        assert!(result.key.is_some());
    }
}
