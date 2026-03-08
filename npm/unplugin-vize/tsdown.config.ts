import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/index.ts", "src/esbuild.ts", "src/rollup.ts", "src/webpack.ts"],
  format: "esm",
  dts: true,
  clean: true,
});
