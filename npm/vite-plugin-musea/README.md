# @vizejs/vite-plugin-musea

Vite plugin for Musea - Vue component gallery and documentation.

## Installation

```bash
npm install @vizejs/vite-plugin-musea
```

## Usage

```ts
// vite.config.ts
import { defineConfig } from "vite";
import { musea } from "@vizejs/vite-plugin-musea";

export default defineConfig({
  plugins: [
    musea({
      // Art files pattern
      include: "**/*.art.vue",
      // Output directory
      outDir: ".musea",
    }),
  ],
});
```

## Art File Format

```vue
<!-- Button.art.vue -->
<art title="Button" component="./Button.vue">
  <variant name="Primary" default>
    <Button variant="primary">Click me</Button>
  </variant>
  <variant name="Disabled">
    <Button disabled>Disabled</Button>
  </variant>
</art>
```

## Commands

```bash
# Start dev server
vite dev

# Build gallery
vite build
```

## License

MIT
