import { test } from "node:test";
import path from "node:path";
import { rspack } from "@rspack/core";
import "./test/setup.ts";
import { VizePlugin } from "./plugin/index.js";
import {
  normalizeSnapshot,
  packageRoot,
  prepareOutputDir,
  resolveFixturePath,
} from "./test/helpers.js";

function runCompiler(compiler: ReturnType<typeof rspack>) {
  return new Promise<
    NonNullable<Parameters<Parameters<typeof compiler.run>[0]>[1]>
  >((resolve, reject) => {
    compiler!.run((error, stats) => {
      compiler!.close((closeError) => {
        if (error || closeError) {
          reject(error ?? closeError);
          return;
        }

        if (!stats) {
          reject(new Error("Rspack did not return stats"));
          return;
        }

        resolve(stats);
      });
    });
  });
}

void test("rspack builds a Vue SFC with auto-inject mode", async (t) => {
  const compiler = rspack({
    mode: "development",
    devtool: false,
    context: resolveFixturePath("basic", "."),
    entry: {
      main: resolveFixturePath("basic", "entry.ts"),
    },
    output: {
      path: prepareOutputDir("integration"),
      filename: "bundle.js",
      clean: true,
    },
    externals: {
      vue: "vue",
    },
    experiments: {
      css: true,
    },
    infrastructureLogging: {
      level: "error",
    },
    resolve: {
      extensions: ["...", ".ts", ".js", ".vue"],
    },
    module: {
      rules: [
        // TypeScript support
        {
          test: /\.ts$/,
          loader: "builtin:swc-loader",
          options: {
            jsc: { parser: { syntax: "typescript" } },
          },
        },
        // TypeScript post-processing for .vue files
        {
          test: /\.vue$/,
          resourceQuery: { not: [/type=/] },
          enforce: "post" as const,
          loader: "builtin:swc-loader",
          options: {
            jsc: { parser: { syntax: "typescript" } },
          },
          type: "javascript/auto",
        },
        // Simple .vue rule — VizePlugin auto-injects style sub-request handling
        {
          test: /\.vue$/,
          use: [
            {
              loader: path.join(packageRoot, "dist", "loader", "index.js"),
            },
          ],
        },
      ],
    },
    plugins: [
      new VizePlugin({
        css: {
          native: true,
        },
      }),
    ],
  });

  const stats = await runCompiler(compiler);
  const info = stats.toJson({
    all: false,
    errors: true,
    assets: true,
  });

  if (stats.hasErrors()) {
    throw new Error(JSON.stringify(info.errors, null, 2));
  }

  const assets = Object.fromEntries(
    Object.entries(stats.compilation.assets)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([name, asset]) => [
        name,
        normalizeSnapshot(asset.source().toString()),
      ]),
  );

  t.assert.snapshot(JSON.stringify(assets, null, 2));
});

void test("rspack rewrites template asset URLs into import bindings", async (t) => {
  const compiler = rspack({
    mode: "development",
    devtool: false,
    context: resolveFixturePath("asset-url", "."),
    entry: {
      main: resolveFixturePath("asset-url", "entry.ts"),
    },
    output: {
      path: prepareOutputDir("asset-url"),
      filename: "bundle.js",
      clean: true,
    },
    externals: {
      vue: "vue",
    },
    experiments: {
      css: true,
    },
    infrastructureLogging: {
      level: "error",
    },
    resolve: {
      extensions: ["...", ".ts", ".js", ".vue"],
    },
    module: {
      rules: [
        {
          test: /\.(png|jpe?g|gif|svg|mp4|webm)$/,
          type: "asset/resource",
        },
        // TypeScript support
        {
          test: /\.ts$/,
          loader: "builtin:swc-loader",
          options: {
            jsc: { parser: { syntax: "typescript" } },
          },
        },
        // TypeScript post-processing for .vue files
        {
          test: /\.vue$/,
          resourceQuery: { not: [/type=/] },
          enforce: "post" as const,
          loader: "builtin:swc-loader",
          options: {
            jsc: { parser: { syntax: "typescript" } },
          },
          type: "javascript/auto",
        },
        // Simple .vue rule — VizePlugin auto-injects style sub-request handling
        {
          test: /\.vue$/,
          use: [
            {
              loader: path.join(packageRoot, "dist", "loader", "index.js"),
            },
          ],
        },
      ],
    },
    plugins: [
      new VizePlugin({
        css: {
          native: true,
        },
      }),
    ],
  });

  const stats = await runCompiler(compiler);
  const info = stats.toJson({
    all: false,
    errors: true,
    warnings: true,
  });

  // Should compile without errors
  if (stats.hasErrors()) {
    throw new Error(JSON.stringify(info.errors, null, 2));
  }

  const bundleJs =
    stats.compilation.assets["bundle.js"]?.source().toString() ?? "";
  const normalized = normalizeSnapshot(bundleJs);

  // Static relative URLs must become import bindings (not remain as string literals)
  t.assert.ok(
    !normalized.includes('"./logo.png"'),
    '"./logo.png" string literal should have been replaced by import binding',
  );
  t.assert.ok(
    !normalized.includes('"./intro.mp4"'),
    '"./intro.mp4" string literal should have been replaced by import binding',
  );
  t.assert.ok(
    !normalized.includes('"./poster.jpg"'),
    '"./poster.jpg" string literal should have been replaced by import binding',
  );

  // External URL must remain unchanged (not transformed into import)
  t.assert.ok(
    normalized.includes("https://cdn.example.com/external.png"),
    "external URL should remain as-is",
  );

  // Dynamic binding value must not be collected as asset URL
  t.assert.ok(
    !normalized.includes("import") ||
      !normalized.includes('from "dynamic.png"'),
    "dynamic binding value should not become an import",
  );
});
