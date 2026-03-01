//! Helper functions and constants for virtual TypeScript generation.
//!
//! Contains utility functions for type declarations, event type mapping,
//! identifier conversion, and template context generation.

use super::types::VirtualTsOptions;
use vize_carton::append;
use vize_carton::String;

/// Vue compiler macros - these are defined inside setup scope, NOT globally.
/// This ensures they're only valid within <script setup>.
/// Parameters and type parameters are prefixed with _ to avoid "unused" warnings.
pub(crate) const VUE_SETUP_COMPILER_MACROS: &str = r#"  // Compiler macros (only valid in setup scope, not global)
  // Emit type helper: converts { event: [args] } to callable emit function
  type __EmitFn<T> = T extends Record<string, any[]> ? <K extends keyof T>(event: K, ...args: T[K]) => void : T;
  function defineProps<_T = unknown>(): _T { return undefined as unknown as _T; }
  function defineEmits<_T = unknown>(): __EmitFn<_T> { return (() => {}) as any; }
  function defineExpose<_T = unknown>(_exposed?: _T): void { void _exposed; }
  function defineModel<_T = unknown>(_name?: string, _options?: any): _T { void _name; void _options; return undefined as unknown as _T; }
  function defineSlots<_T = unknown>(): _T { return undefined as unknown as _T; }
  function withDefaults<_T = unknown, _D = unknown>(_props: _T, _defaults: _D): _T & _D { void _props; void _defaults; return undefined as unknown as _T & _D; }
  function useTemplateRef<_T extends Element | import('vue').ComponentPublicInstance = Element>(_key: string): import('vue').ShallowRef<_T | null> { void _key; return undefined as unknown as import('vue').ShallowRef<_T | null>; }
  // Mark compiler macros as used
  void defineProps; void defineEmits; void defineExpose; void defineModel; void defineSlots; void withDefaults; void useTemplateRef;"#;

/// ImportMeta augmentation for Vite/Nuxt projects.
/// Uses `declare global` to merge with the built-in ImportMeta interface,
/// so `import.meta.client`, `import.meta.env`, etc. are recognized.
pub(crate) const IMPORT_META_AUGMENTATION: &str = r#"// ImportMeta augmentation (Vite/Nuxt)
declare global {
  interface ImportMeta {
    readonly env: Record<string, string | boolean | undefined>;
    readonly client: boolean;
    readonly server: boolean;
    readonly dev: boolean;
    readonly prod: boolean;
    readonly ssr: boolean;
    readonly hot?: {
      readonly data: any;
      accept(): void;
      accept(cb: (mod: any) => void): void;
      accept(dep: string, cb: (mod: any) => void): void;
      accept(deps: readonly string[], cb: (mods: any[]) => void): void;
      dispose(cb: (data: any) => void): void;
      decline(): void;
      invalidate(message?: string): void;
      on(event: string, cb: (...args: any[]) => void): void;
    };
    glob(pattern: string, options?: any): Record<string, any>;
    glob(pattern: string[], options?: any): Record<string, any>;
  }
}
"#;

/// Generate Vue template context declarations dynamically.
/// Includes Vue core globals ($attrs, $slots, $refs, $emit) and
/// user-configurable plugin globals ($t, $route, etc.).
pub(crate) fn generate_template_context(options: &VirtualTsOptions) -> String {
    let mut ctx = String::default();

    // Vue core globals (always present)
    ctx.push_str("    // Vue instance context (available in template)\n");
    ctx.push_str("    const $attrs: Record<string, unknown> = {} as any;\n");
    ctx.push_str("    const $slots: Record<string, (...args: any[]) => any> = {} as any;\n");
    ctx.push_str("    const $refs: Record<string, any> = {} as any;\n");
    ctx.push_str("    const $emit: (...args: any[]) => void = (() => {}) as any;\n");

    // Plugin globals (configurable)
    if !options.template_globals.is_empty() {
        ctx.push_str("    // Plugin globals (configurable via --globals)\n");
        for global in &options.template_globals {
            append!(
                ctx,
                "    const {}: {} = {};\n",
                global.name,
                global.type_annotation,
                global.default_value
            );
        }
    }

    // Mark all as used
    ctx.push_str("    // Mark template context as used\n");
    ctx.push_str("    void $attrs; void $slots; void $refs; void $emit;\n");
    if !options.template_globals.is_empty() {
        ctx.push_str("    ");
        for (i, global) in options.template_globals.iter().enumerate() {
            if i > 0 {
                ctx.push(' ');
            }
            append!(ctx, "void {};", global.name);
        }
        ctx.push('\n');
    }

    ctx
}

/// Check if a type declaration is complete based on brace depth and declaration kind.
pub(crate) fn is_type_decl_complete(trimmed: &str, brace_depth: i32, is_alias: bool) -> bool {
    if is_alias {
        // Type aliases end with `;` when brace depth is 0
        brace_depth <= 0 && trimmed.ends_with(';')
    } else {
        // Interfaces and enums end with `}` when brace depth returns to 0
        brace_depth <= 0 && (trimmed.ends_with('}') || trimmed.ends_with("};"))
    }
}

/// Check if a trimmed line starts a type declaration that should be at module level.
pub(crate) fn is_type_declaration_start(trimmed: &str) -> bool {
    // Match: interface X, type X =, enum X, export interface X, export type X =, export enum X
    // But NOT: export default, export function, export const, export { ... } from
    // Also NOT: destructured props like `type = "button"` (no identifier after `type`)
    let s = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    if s.starts_with("interface ") || s.starts_with("enum ") {
        return true;
    }
    // For `type` keyword: require a valid identifier after `type `
    // e.g., `type Foo = ...` or `type Foo<T> = ...`
    if let Some(rest) = s.strip_prefix("type ") {
        let rest = rest.trim_start();
        // The next token must be an identifier (starts with letter or _)
        if let Some(first_char) = rest.chars().next() {
            if first_char.is_ascii_alphabetic() || first_char == '_' {
                // Check it's followed by '=' or '<' (generic) eventually
                return rest.contains('=');
            }
        }
    }
    false
}

/// Strip TypeScript `as Type` assertion from a v-for source expression.
/// Returns (source_expression, Option<type_annotation>).
/// e.g., "(expr) as OptionSponsor[]" -> ("(expr)", Some("OptionSponsor[]"))
pub(crate) fn strip_as_assertion(source: &str) -> (&str, Option<&str>) {
    // Look for ` as ` in the source, but be careful with nested expressions.
    // We scan from the end to find the last top-level ` as `.
    let trimmed = source.trim();

    // Simple approach: find the last ` as ` that is not inside parentheses
    let mut paren_depth = 0i32;
    let bytes = trimmed.as_bytes();
    let mut last_as_pos = None;

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => paren_depth += 1,
            b')' => paren_depth -= 1,
            b' ' if paren_depth == 0 => {
                // Check for " as "
                if i + 4 <= bytes.len() && &bytes[i..i + 4] == b" as " {
                    last_as_pos = Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }

    if let Some(pos) = last_as_pos {
        let expr = trimmed[..pos].trim();
        let type_ann = trimmed[pos + 4..].trim();
        if !type_ann.is_empty() {
            return (expr, Some(type_ann));
        }
    }

    (trimmed, None)
}

/// Get the TypeScript event type for a DOM event name.
/// Returns the specific event interface (MouseEvent, KeyboardEvent, etc.)
pub(crate) fn get_dom_event_type(event_name: &str) -> &'static str {
    match event_name {
        // Mouse events
        "click" | "dblclick" | "mousedown" | "mouseup" | "mousemove" | "mouseenter"
        | "mouseleave" | "mouseover" | "mouseout" | "contextmenu" => "MouseEvent",

        // Pointer events
        "pointerdown" | "pointerup" | "pointermove" | "pointerenter" | "pointerleave"
        | "pointerover" | "pointerout" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" => "PointerEvent",

        // Touch events
        "touchstart" | "touchend" | "touchmove" | "touchcancel" => "TouchEvent",

        // Keyboard events
        "keydown" | "keyup" | "keypress" => "KeyboardEvent",

        // Focus events
        "focus" | "blur" | "focusin" | "focusout" => "FocusEvent",

        // Input events
        "input" | "beforeinput" => "InputEvent",

        // Composition events
        "compositionstart" | "compositionend" | "compositionupdate" => "CompositionEvent",

        // Form events
        "submit" => "SubmitEvent",
        "change" => "Event",
        "reset" => "Event",

        // Drag events
        "drag" | "dragstart" | "dragend" | "dragenter" | "dragleave" | "dragover" | "drop" => {
            "DragEvent"
        }

        // Clipboard events
        "cut" | "copy" | "paste" => "ClipboardEvent",

        // Wheel events
        "wheel" => "WheelEvent",

        // Animation events
        "animationstart" | "animationend" | "animationiteration" | "animationcancel" => {
            "AnimationEvent"
        }

        // Transition events
        "transitionstart" | "transitionend" | "transitionrun" | "transitioncancel" => {
            "TransitionEvent"
        }

        // UI events
        "scroll" | "resize" => "Event",

        // Media events
        "play" | "pause" | "ended" | "loadeddata" | "loadedmetadata" | "timeupdate"
        | "volumechange" | "waiting" | "seeking" | "seeked" | "ratechange" | "durationchange"
        | "canplay" | "canplaythrough" | "playing" | "progress" | "stalled" | "suspend"
        | "emptied" | "abort" => "Event",

        // Error/Load events
        "error" => "ErrorEvent",
        "load" => "Event",

        // Selection events
        "select" | "selectionchange" | "selectstart" => "Event",

        // Default fallback
        _ => "Event",
    }
}

/// Convert kebab-case or PascalCase prop name to camelCase.
/// Vue normalizes prop names to camelCase internally.
/// Examples: "my-prop" -> "myProp", "MyProp" -> "myProp"
pub(crate) fn to_camel_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    let mut first = true;

    for c in s.chars() {
        if c == '-' || c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else if first {
            // First character should be lowercase
            result.push(c.to_ascii_lowercase());
            first = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Sanitize a string to be a valid TypeScript identifier.
/// Replaces invalid characters (like ':') with underscores.
/// Examples: "update:title" -> "update_title", "my-event" -> "my_event"
pub(crate) fn to_safe_identifier(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
