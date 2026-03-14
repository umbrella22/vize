<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_atelier_dom/logo.svg" alt="vize_atelier_dom logo" width="120" height="120">
</p>

<h1 align="center">vize_atelier_dom</h1>

<p align="center">
  <strong>The DOM Workshop - Vue template compiler for Virtual DOM output</strong>
</p>

---

## Name Origin

**Atelier** (/ˌætəlˈjeɪ/) is a French word for an artist's workshop or studio - a dedicated space where master craftspeople practice their specific art form. Just as Renaissance artists had specialized ateliers for painting, sculpture, or metalwork, `vize_atelier_dom` is a specialized workshop focused on one specific compilation target: the Virtual DOM.

In the art world, an atelier:

- **Specializes** - Focuses on a particular craft or technique
- **Transforms** - Converts raw materials into refined artworks
- **Masters** - Applies deep expertise to its domain

Similarly, `vize_atelier_dom` provides:

- **DOM-specific transforms** - v-model, v-show, v-text, v-html, v-on
- **Virtual DOM codegen** - Generates optimized render functions
- **Platform-specific optimization** - Browser DOM-aware compilation

## Features

### DOM Directives

Specialized transforms for DOM-specific Vue directives:

- **v-model** - Two-way binding with input elements
- **v-show** - CSS display toggling
- **v-text** - Text content binding
- **v-html** - Inner HTML binding
- **v-on** - Event handling with modifiers

### Integration with Vize

```rust
use vize_atelier_dom::{compile_template_with_options, DomCompilerOptions};
use vize_carton::Bump;

let allocator = Bump::new();
let (root, errors, result) = compile_template_with_options(
    &allocator,
    "<div v-show=\"visible\">{{ message }}</div>",
    DomCompilerOptions::default()
);
```

## Part of the Vize Art Collection

`vize_atelier_dom` is part of the Vize compiler's art-themed crate collection:

| Crate                | Art Term                | Role                          |
| -------------------- | ----------------------- | ----------------------------- |
| vize_carton          | Carton (Portfolio Case) | Shared utilities & allocator  |
| vize_atelier_core    | -                       | AST & Parser (core)           |
| **vize_atelier_dom** | **Atelier (Workshop)**  | **DOM compiler (this crate)** |
| vize_atelier_vapor   | Atelier (Workshop)      | Vapor mode compiler           |
| vize_atelier_sfc     | Atelier (Workshop)      | SFC compiler                  |
| vize_vitrine         | Vitrine (Display Case)  | Bindings (Node.js/WASM)       |

## License

MIT License
