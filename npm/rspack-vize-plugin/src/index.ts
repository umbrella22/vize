/** @vizejs/rspack-plugin — Rspack plugin for Vue SFC compilation powered by Vize. */

// Plugin
export { VizePlugin } from "./plugin/index.js";
export { applyRuleCloning } from "./plugin/ruleCloning.js";
export type { RuleCloningResult } from "./plugin/ruleCloning.js";
export type { VizeRspackPluginOptions } from "./types/index.js";

// Loaders (for direct import)
export { default as vizeLoader } from "./loader/index.js";
export { default as vizeStyleLoader } from "./loader/style-loader.js";
export { default as vizeScopeLoader } from "./loader/scope-loader.js";
export type { VizeLoaderOptions, VizeStyleLoaderOptions } from "./types/index.js";

// Shared utilities (optional export for advanced usage)
export {
  generateScopeId,
  extractStyleBlocks,
  extractCustomBlocks,
  extractSrcInfo,
  inlineSrcBlocks,
  addScopeToCssFallback,
  matchesPattern,
} from "./shared/utils.js";

export { genHotReloadCode } from "./shared/hotReload.js";

export { compileFile, generateOutput, clearCompilationCache } from "./shared/compiler.js";

// Types
export type {
  CompiledModule,
  StyleBlockInfo,
  CustomBlockInfo,
  SfcSrcInfo,
  SfcCompileOptionsNapi,
  SfcCompileResultNapi,
  LoaderEntry,
} from "./types/index.js";
