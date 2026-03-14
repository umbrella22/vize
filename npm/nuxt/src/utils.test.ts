import assert from "node:assert";
import {
  buildNuxtCompilerOptions,
  buildNuxtDevAssetBase,
  isVizeVirtualVueModuleId,
  normalizeVizeVirtualVueModuleId,
} from "./utils.ts";

assert.strictEqual(
  buildNuxtDevAssetBase("/", "/_nuxt/"),
  "/_nuxt/",
  "default Nuxt dev assets should stay under /_nuxt/",
);

assert.strictEqual(
  buildNuxtDevAssetBase("/2025/", "/_nuxt/"),
  "/2025/_nuxt/",
  "Nuxt baseURL should prefix buildAssetsDir",
);

assert.strictEqual(
  buildNuxtDevAssetBase("/preview", "_assets"),
  "/preview/_assets/",
  "missing slashes should be normalized",
);

assert.deepStrictEqual(
  buildNuxtCompilerOptions("/repo/app", "/2026/", "/_nuxt/"),
  {
    devUrlBase: "/2026/_nuxt/",
    root: "/repo/app",
  },
  "Nuxt compiler options should pin Vize root to the app root so vize.config.ts is discovered",
);

assert.equal(
  isVizeVirtualVueModuleId("\0vize-ssr:/repo/app/components/Foo.vue.ts"),
  true,
  "SSR virtual Vue modules should stay eligible for Nuxt bridge transforms",
);

assert.equal(
  normalizeVizeVirtualVueModuleId("\0vize-ssr:/repo/app/components/Foo.vue.ts"),
  "/repo/app/components/Foo.vue",
  "Nuxt bridge normalization should strip only the virtual .ts suffix",
);

console.log("✅ nuxt utils tests passed!");
