import { defineConfig } from "vite-plus";
import { vize } from "@vizejs/vite-plugin";

export default defineConfig({
  base: process.env.CI ? "/play/" : "/",
  plugins: [vize({ vapor: true })],
  server: {
    port: 5180,
    strictPort: false,
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  optimizeDeps: {
    include: ["monaco-editor", "shiki", "prettier/plugins/html"],
    exclude: ["vize-wasm"],
  },
});
