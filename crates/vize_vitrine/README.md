<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_vitrine/logo.svg" alt="vize_vitrine logo" width="120" height="120">
</p>

<h1 align="center">vize_vitrine</h1>

<p align="center">
  <strong>The display case for Vize - Node.js & WebAssembly bindings</strong>
</p>

---

## Name Origin

**Vitrine** (/vɪˈtriːn/) is a glass display case used in museums and galleries to showcase precious artworks and artifacts to the public. Just as a vitrine presents treasures to visitors while protecting them, `vize_vitrine` exposes the Vize compiler's capabilities to the JavaScript ecosystem through carefully crafted bindings.

In the art world, a vitrine:

- **Displays** - Presents works for public viewing
- **Protects** - Provides a safe interface to valuable items
- **Illuminates** - Makes art accessible and visible

Similarly, `vize_vitrine` provides:

- **Node.js bindings** - Native performance via NAPI
- **WebAssembly bindings** - Browser-compatible compilation
- **Safe interfaces** - Type-safe APIs for JavaScript/TypeScript

## Features

### Node.js (Native)

High-performance native bindings using NAPI:

```javascript
const { compileSfc } = require("@vize/native");

const { code } = compileSfc(
  `
  <template>
    <div>{{ msg }}</div>
  </template>
  <script setup>
  const msg = 'Hello!'
  </script>
`,
  { filename: "App.vue" },
);
```

### WebAssembly (Browser)

Cross-platform WASM bindings:

```javascript
import init, { compileSfc } from "@vize/wasm";

await init();
const { code } = compileSfc(`...`, { filename: "App.vue" });
```

### Batch Compilation

Compile multiple files efficiently:

```javascript
const { compileGlob } = require("@vize/native");

const results = compileGlob("src/**/*.vue", {
  threads: 4,
  format: "js",
});
```

## Build

```bash
# Native bindings (Node.js)
mise run build:native

# WASM bindings (Browser)
mise run build:wasm-web
```

## Part of the Vize Art Collection

`vize_vitrine` is part of the Vize compiler's art-themed crate collection:

| Crate             | Art Term                       | Role                         |
| ----------------- | ------------------------------ | ---------------------------- |
| vize_carton       | Carton (Portfolio Case)        | Shared utilities & allocator |
| vize_relief       | Relief (Sculpted Surface)      | AST structures               |
| vize_atelier_core | Armature (Sculpture Framework) | Parser & tokenizer           |
| vize_atelier      | Atelier (Artist's Studio)      | Compilers                    |
| **vize_vitrine**  | **Vitrine (Display Case)**     | **Bindings (this crate)**    |

## License

MIT License
