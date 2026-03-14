<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_atelier_core/logo.svg" alt="vize_atelier_core logo" width="120" height="120">
</p>

<h1 align="center">vize_atelier_core</h1>

<p align="center">
  <strong>The Armature - Structural framework for Vue template compilation</strong>
</p>

---

## Name Origin

**Armature** (/ˈɑːrmətʃər/) is the internal framework or skeleton that supports a sculpture during its creation. In traditional sculpture, an armature is typically made of wire or metal, providing the essential structure around which clay, plaster, or other materials are built. In animation and digital art, armatures (or "rigs") provide the skeletal system for character movement.

In the art world, an armature:

- **Supports** - Provides structural foundation for the artwork
- **Shapes** - Defines the basic form and proportions
- **Enables** - Makes complex forms possible to construct

Similarly, `vize_atelier_core` provides:

- **AST definitions** - The structural foundation for Vue templates
- **Parsing** - Tokenization and parsing of Vue template syntax
- **Transforms** - Pipeline for processing and optimizing the AST
- **Code generation** - Converting AST back to JavaScript code

## Features

- Vue template AST definition and manipulation
- High-performance tokenizer and parser
- Comprehensive transform pipeline
- Code generation with runtime helper support

## Part of the Vize Art Collection

`vize_atelier_core` is the core structural framework of the Vize compiler's art-themed crate collection:

| Crate                 | Art Term                | Role                          |
| --------------------- | ----------------------- | ----------------------------- |
| vize_carton           | Carton (Portfolio Case) | Shared utilities & allocator  |
| **vize_atelier_core** | **Armature (Skeleton)** | **AST & Parser (this crate)** |
| vize_atelier_dom      | Atelier (Workshop)      | DOM compiler                  |
| vize_atelier_vapor    | Atelier (Workshop)      | Vapor compiler                |
| vize_atelier_sfc      | Atelier (Workshop)      | SFC compiler                  |
| vize_vitrine          | Vitrine (Display Case)  | Bindings (Node.js/WASM)       |
| vize_canon            | Canon (Standard)        | Type checker                  |
| vize_glyph            | Glyph (Letterform)      | Formatter                     |
| vize_patina           | Patina (Aged Surface)   | Linter                        |
| vize_cli              | -                       | CLI                           |

## License

MIT License
