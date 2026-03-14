<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_atelier_vapor/logo.svg" alt="vize_atelier_vapor logo" width="120" height="120">
</p>

<h1 align="center">vize_atelier_vapor</h1>

<p align="center">
  <strong>The Vapor Workshop - Vue template compiler for Vapor mode output</strong>
</p>

---

## Name Origin

**Atelier** (/ˌætəlˈjeɪ/) is a French word for an artist's workshop or studio - a dedicated space where master craftspeople practice their specific art form. While `vize_atelier_dom` focuses on Virtual DOM compilation, `vize_atelier_vapor` is a specialized workshop for the cutting-edge Vapor mode - Vue's next-generation rendering approach.

In the art world, an atelier:

- **Specializes** - Focuses on a particular craft or technique
- **Innovates** - Pushes boundaries with new techniques
- **Masters** - Applies deep expertise to its domain

Similarly, `vize_atelier_vapor` provides:

- **Vapor-specific transforms** - Optimized for direct DOM manipulation
- **Fine-grained reactivity** - No virtual DOM diffing overhead
- **Template-based optimization** - Static analysis at compile time

## Features

### Vapor Mode Compilation

Generates highly optimized code for Vue's Vapor rendering mode:

- **Direct DOM operations** - No virtual DOM overhead
- **Fine-grained updates** - Only update what changes
- **Static template extraction** - Pre-compile static portions

### Example

```rust
use vize_atelier_vapor::{compile_vapor, VaporCompilerOptions};
use vize_carton::Bump;

let allocator = Bump::new();
let result = compile_vapor(
    &allocator,
    "<div>{{ message }}</div>",
    VaporCompilerOptions::default()
);

println!("{}", result.code);
```

### Output Example

```javascript
import { _template, _setText } from "vue/vapor";

const t0 = _template("<div></div>");

export default () => {
  const n0 = t0();
  _setText(n0, message);
  return n0;
};
```

## Part of the Vize Art Collection

`vize_atelier_vapor` is part of the Vize compiler's art-themed crate collection:

| Crate                  | Art Term                | Role                            |
| ---------------------- | ----------------------- | ------------------------------- |
| vize_carton            | Carton (Portfolio Case) | Shared utilities & allocator    |
| vize_atelier_core      | -                       | AST & Parser (core)             |
| vize_atelier_dom       | Atelier (Workshop)      | DOM compiler                    |
| **vize_atelier_vapor** | **Atelier (Workshop)**  | **Vapor compiler (this crate)** |
| vize_atelier_sfc       | Atelier (Workshop)      | SFC compiler                    |
| vize_vitrine           | Vitrine (Display Case)  | Bindings (Node.js/WASM)         |

## License

MIT License
