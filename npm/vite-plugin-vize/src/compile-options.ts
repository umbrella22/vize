import type { BatchCompileOptionsNapi, SfcCompileOptionsNapi } from "./types.js";
import { generateScopeId } from "./utils/index.js";

export interface CompileFileOptions {
  sourceMap: boolean;
  ssr: boolean;
  vapor: boolean;
}

export interface CompileBatchOptions {
  ssr: boolean;
  vapor: boolean;
}

export function buildCompileFileOptions(
  filePath: string,
  source: string,
  options: CompileFileOptions,
): SfcCompileOptionsNapi {
  const scopeId = /<style[^>]*\bscoped\b/.test(source)
    ? `data-v-${generateScopeId(filePath)}`
    : undefined;

  return {
    filename: filePath,
    sourceMap: options.sourceMap,
    ssr: options.ssr,
    vapor: options.vapor,
    scopeId,
  };
}

export function buildCompileBatchOptions(options: CompileBatchOptions): BatchCompileOptionsNapi {
  return {
    ssr: options.ssr,
    vapor: options.vapor,
  };
}
