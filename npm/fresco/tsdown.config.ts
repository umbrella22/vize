import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/index.ts", "src/components/index.ts", "src/composables/index.ts"],
  format: "esm",
  dts: true,
  clean: true,
});
