<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_atelier_sfc/logo.svg" alt="vize_atelier_sfc logo" width="120" height="120">
</p>

<h1 align="center">vize_atelier_sfc</h1>

<p align="center">
  <strong>The SFC Workshop - Vue Single File Component compiler</strong>
</p>

---

## Name Origin

**Atelier** (/ˌætəlˈjeɪ/) is a French word for an artist's workshop or studio. While `vize_atelier_dom` and `vize_atelier_vapor` focus on template compilation for specific rendering modes, `vize_atelier_sfc` is the master workshop that orchestrates the complete Single File Component compilation process.

In the art world, an atelier:

- **Orchestrates** - Coordinates multiple artistic disciplines
- **Integrates** - Combines different mediums into a unified work
- **Completes** - Produces finished, exhibition-ready pieces

Similarly, `vize_atelier_sfc` provides:

- **SFC Parsing** - Parse `.vue` files into descriptor blocks
- **Script Compilation** - Process `<script>` and `<script setup>` blocks
- **Template Integration** - Delegate to DOM or Vapor ateliers
- **Style Processing** - Scoped CSS with LightningCSS

## Features

### SFC Parsing

```rust
use vize_atelier_sfc::{parse_sfc, SfcParseOptions};

let source = r#"
<script setup>
const msg = 'Hello!'
</script>
<template>
  <div>{{ msg }}</div>
</template>
<style scoped>
div { color: red; }
</style>
"#;

let descriptor = parse_sfc(source, SfcParseOptions::default()).unwrap();
```

### Full SFC Compilation

```rust
use vize_atelier_sfc::{compile_sfc, SfcCompileOptions};

let result = compile_sfc(&descriptor, SfcCompileOptions::default()).unwrap();
println!("{}", result.code);
```

### Script Setup Support

- `defineProps` / `withDefaults`
- `defineEmits`
- `defineExpose`
- `defineModel`
- `defineSlots`
- `defineOptions`
- Props destructuring with reactivity transform

### CSS Features

- Scoped CSS with data attributes
- `:deep()`, `:global()`, `:slotted()` pseudo-selectors
- CSS `v-bind()` for reactive styles
- CSS minification and autoprefixing via LightningCSS

## Part of the Vize Art Collection

`vize_atelier_sfc` is part of the Vize compiler's art-themed crate collection:

| Crate                | Art Term                | Role                          |
| -------------------- | ----------------------- | ----------------------------- |
| vize_carton          | Carton (Portfolio Case) | Shared utilities & allocator  |
| vize_atelier_core    | -                       | AST & Parser (core)           |
| vize_atelier_dom     | Atelier (Workshop)      | DOM compiler                  |
| vize_atelier_vapor   | Atelier (Workshop)      | Vapor compiler                |
| **vize_atelier_sfc** | **Atelier (Workshop)**  | **SFC compiler (this crate)** |
| vize_vitrine         | Vitrine (Display Case)  | Bindings (Node.js/WASM)       |

## License

MIT License
