<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_canon/logo.svg" alt="vize_canon logo" width="120" height="120">
</p>

<h1 align="center">vize_canon</h1>

<p align="center">
  <strong>The Canon - TypeScript type checking for Vue SFCs</strong>
</p>

---

## Name Origin

**Canon** (/ˈkænən/) in art refers to a set of ideal proportions or standards that define aesthetic perfection. Ancient Greek sculptors like Polykleitos established the "Canon" - mathematical ratios for the ideal human figure that became the standard for classical sculpture.

In the art world, a canon:

- **Defines standards** - Establishes rules for correctness
- **Measures perfection** - Evaluates against ideal proportions
- **Guides creation** - Ensures harmony and balance

Similarly, `vize_canon` provides:

- **Type checking** - Validates TypeScript types in Vue SFCs
- **Error detection** - Identifies type mismatches and inconsistencies
- **Code correctness** - Ensures your Vue components follow type contracts

## Features

- **Template type checking** - Validate expressions in `{{ }}`
- **Directive validation** - Type-check `v-bind`, `v-on`, etc.
- **Props inference** - Infer component prop types
- **Emit validation** - Check event handler types

## Usage

```rust
use vize_canon::{TypeChecker, TypeContext};

let checker = TypeChecker::new();
let ctx = TypeContext::from_sfc(&descriptor);

// Check template for type errors
let diagnostics = checker.check_template(&ctx);

// Get type at position
if let Some(info) = checker.get_type_at(&ctx, offset) {
    println!("Type: {}", info.display());
}
```

## Error Codes

| Code | Description             |
| ---- | ----------------------- |
| 2304 | Cannot find name        |
| 2339 | Property does not exist |
| 2345 | Argument type mismatch  |

## Part of the Vize Art Collection

`vize_canon` is part of the Vize compiler's art-themed crate collection:

| Crate              | Art Term                | Role                          |
| ------------------ | ----------------------- | ----------------------------- |
| vize_carton        | Carton (Portfolio Case) | Shared utilities & allocator  |
| vize_atelier_core  | -                       | AST & Parser (core)           |
| vize_atelier_dom   | Atelier (Workshop)      | DOM compiler                  |
| vize_atelier_vapor | Atelier (Workshop)      | Vapor compiler                |
| vize_atelier_sfc   | Atelier (Workshop)      | SFC compiler                  |
| vize_vitrine       | Vitrine (Display Case)  | Bindings (Node.js/WASM)       |
| **vize_canon**     | **Canon (Standard)**    | **Type checker (this crate)** |

## License

MIT License
