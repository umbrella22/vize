<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_carton/logo.svg" alt="vize_carton logo" width="120" height="120">
</p>

<h1 align="center">vize_carton</h1>

<p align="center">
  <strong>The artist's toolbox for Vize compiler</strong>
</p>

---

## Name Origin

**Carton** (/kɑːˈtɒn/) is an artist's portfolio case or art supply box - a container that holds all the essential tools and materials an artist needs for their work. Just as a carton organizes brushes, paints, and canvases for an artist, `vize_carton` organizes the fundamental utilities and data structures needed for the Vize compiler.

In the art world, a carton typically contains:

- **Brushes & Tools** - The instruments for creating
- **Paints & Pigments** - The raw materials
- **Canvas preparation supplies** - The foundation

Similarly, `vize_carton` provides:

- **Arena Allocator** - Efficient memory management for AST construction
- **Shared Data Structures** - Common types used across the compiler
- **Utility Functions** - Helper functions for string manipulation, tag validation, etc.

## Features

### Arena Allocation

High-performance arena-based memory allocation optimized for compiler workloads:

```rust
use vize_carton::{Allocator, Box, Vec};

let allocator = Allocator::default();

// Allocate values in the arena
let boxed = Box::new_in(42, allocator.as_bump());
let mut vec = Vec::new_in(allocator.as_bump());
vec.push(1);
vec.push(2);
```

### Shared Utilities

#### DOM Tag Configuration

```rust
use vize_carton::{is_html_tag, is_svg_tag, is_void_tag};

assert!(is_html_tag("div"));
assert!(is_svg_tag("path"));
assert!(is_void_tag("br"));
```

#### String Transformations

```rust
use vize_carton::{camelize, hyphenate, capitalize};

assert_eq!(camelize("foo-bar"), "fooBar");
assert_eq!(hyphenate("fooBar"), "foo-bar");
assert_eq!(capitalize("hello"), "Hello");
```

#### Optimization Flags

```rust
use vize_carton::{PatchFlags, ShapeFlags, SlotFlags};

let flags = PatchFlags::TEXT | PatchFlags::CLASS;
assert!(flags.contains(PatchFlags::TEXT));
```

## Part of the Vize Art Collection

`vize_carton` is part of the Vize compiler's art-themed crate collection:

| Crate             | Art Term                       | Role                         |
| ----------------- | ------------------------------ | ---------------------------- |
| **vize_carton**   | Carton (Portfolio Case)        | Shared utilities & allocator |
| vize_relief       | Relief (Sculpted Surface)      | AST structures               |
| vize_atelier_core | Armature (Sculpture Framework) | Parser & tokenizer           |
| vize_atelier      | Atelier (Artist's Studio)      | Compilers                    |
| vize_vitrine      | Vitrine (Display Case)         | Bindings                     |

## License

MIT License
