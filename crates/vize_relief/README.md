<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_relief/logo.svg" alt="vize_relief logo" width="120" height="120">
</p>

<h1 align="center">vize_relief</h1>

<p align="center">
  <strong>The Relief - Sculptured AST surface for Vue templates</strong>
</p>

---

## Name Origin

**Relief** (/rɪˈliːf/) is a sculptural technique where figures project from a flat background, creating depth and dimension. Like bas-relief, mezzo-relief, and high-relief in sculpture, the AST "relief" reveals the structure hidden within Vue template source code.

In the art world, relief:

- **Projects** - Raises forms from a flat surface
- **Reveals** - Exposes depth and structure
- **Defines** - Creates clear, distinct forms

Similarly, `vize_relief` provides:

- **AST definitions** - Complete Vue template node types
- **Type safety** - Strongly typed node structures
- **Arena allocation** - Zero-copy JavaScript interop

## Features

- Complete Vue template AST node definitions
- Element, Text, Comment, Interpolation nodes
- Directive and Attribute representations
- Control flow nodes (If, For, etc.)
- Code generation node types
- Serialization support with serde

## Part of the Vize Art Collection

`vize_relief` is the AST foundation of the Vize compiler's art-themed crate collection:

| Crate              | Art Term                        | Role                             |
| ------------------ | ------------------------------- | -------------------------------- |
| vize_carton        | Carton (Portfolio Case)         | Shared utilities & allocator     |
| **vize_relief**    | **Relief (Sculptured Surface)** | **AST definitions (this crate)** |
| vize_atelier_core  | Atelier (Workshop)              | Parser, transforms, codegen      |
| vize_atelier_dom   | Atelier (Workshop)              | DOM compiler                     |
| vize_atelier_vapor | Atelier (Workshop)              | Vapor compiler                   |
| vize_atelier_sfc   | Atelier (Workshop)              | SFC compiler                     |
| vize_vitrine       | Vitrine (Display Case)          | Bindings (Node.js/WASM)          |
| vize_canon         | Canon (Standard)                | Type checker                     |
| vize_glyph         | Glyph (Letterform)              | Formatter                        |
| vize_patina        | Patina (Aged Surface)           | Linter                           |

## License

MIT License
