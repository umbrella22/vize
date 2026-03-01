/**
 * Virtual module ID management and dynamic import rewriting for Vize.
 *
 * Handles the mapping between real .vue file paths and their virtual module
 * counterparts, as well as rewriting dynamic template imports for alias resolution.
 */

import path from "node:path";
import fs from "node:fs";

// Virtual module prefixes and constants
export const LEGACY_VIZE_PREFIX = "\0vize:";
export const VIRTUAL_CSS_MODULE = "virtual:vize-styles";
export const RESOLVED_CSS_MODULE = "\0vize:all-styles.css";

export interface DynamicImportAliasRule {
  fromPrefix: string;
  toPrefix: string;
}

/** Check if a module ID is a vize-compiled virtual module */
export function isVizeVirtual(id: string): boolean {
  return id.startsWith("\0") && id.endsWith(".vue.ts");
}

/** Create a virtual module ID from a real .vue file path */
export function toVirtualId(realPath: string): string {
  return "\0" + realPath + ".ts";
}

/** Extract the real .vue file path from a virtual module ID */
export function fromVirtualId(virtualId: string): string {
  // Strip \0 prefix and .ts suffix
  return virtualId.slice(1, -3);
}

export function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function toBrowserImportPrefix(replacement: string): string {
  const normalized = replacement.replace(/\\/g, "/");
  if (normalized.startsWith("/@fs/")) {
    return normalized;
  }
  // Absolute filesystem alias targets should be served via /@fs in browser imports.
  if (path.isAbsolute(replacement) && fs.existsSync(replacement)) {
    return `/@fs${normalized}`;
  }
  return normalized;
}

export function normalizeFsIdForBuild(id: string): string {
  const [pathPart, queryPart] = id.split("?");
  if (!pathPart.startsWith("/@fs/")) {
    return id;
  }
  const normalizedPath = pathPart.slice(4); // strip '/@fs'
  return queryPart ? `${normalizedPath}?${queryPart}` : normalizedPath;
}

export function rewriteDynamicTemplateImports(
  code: string,
  aliasRules: DynamicImportAliasRule[],
): string {
  let rewritten = code;

  // Normalize alias-based template literal imports (e.g. `@/foo/${x}.svg`) to browser paths.
  for (const rule of aliasRules) {
    const pattern = new RegExp(`\\bimport\\s*\\(\\s*\`${escapeRegExp(rule.fromPrefix)}`, "g");
    rewritten = rewritten.replace(pattern, `import(/* @vite-ignore */ \`${rule.toPrefix}`);
  }

  // Dynamic template imports are intentionally runtime-resolved: mark them to silence
  // Vite's static analysis warning while keeping runtime behavior.
  rewritten = rewritten.replace(/\bimport\s*\(\s*`/g, "import(/* @vite-ignore */ `");

  return rewritten;
}
