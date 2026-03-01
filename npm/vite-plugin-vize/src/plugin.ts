/**
 * Main Vize Vite plugin implementation.
 *
 * Contains the `vize()` factory function that creates the Vite plugin array,
 * including SFC compilation, virtual module handling, HMR, and CSS extraction.
 */

import type { Plugin, ResolvedConfig, ViteDevServer, HmrContext, TransformResult } from "vite";
import { transformWithOxc } from "vite";
import path from "node:path";
import fs from "node:fs";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { glob } from "tinyglobby";

import type { VizeOptions, CompiledModule, ConfigEnv } from "./types.js";
import { compileFile, compileBatch } from "./compiler.js";
import { createFilter, generateOutput, resolveCssImports, type CssAliasRule } from "./utils.js";
import { detectHmrUpdateType, type HmrUpdateType } from "./hmr.js";
import { loadConfig, vizeConfigStore } from "./config.js";
import {
  LEGACY_VIZE_PREFIX,
  VIRTUAL_CSS_MODULE,
  RESOLVED_CSS_MODULE,
  isVizeVirtual,
  toVirtualId,
  fromVirtualId,
  toBrowserImportPrefix,
  normalizeFsIdForBuild,
  rewriteDynamicTemplateImports,
  type DynamicImportAliasRule,
} from "./virtual.js";
import {
  rewriteStaticAssetUrls,
  isBuiltinDefine,
  applyDefineReplacements,
  createLogger,
} from "./transform.js";

export function vize(options: VizeOptions = {}): Plugin[] {
  const cache = new Map<string, CompiledModule>();
  // Collected CSS for production extraction
  const collectedCss = new Map<string, string>();

  let isProduction: boolean;
  let root: string;
  let clientViteBase = "/";
  let serverViteBase = "/";
  let server: ViteDevServer | null = null;
  let filter: (id: string) => boolean;
  let scanPatterns: string[];
  let ignorePatterns: string[];
  let mergedOptions: VizeOptions;
  let initialized = false;
  let dynamicImportAliasRules: DynamicImportAliasRule[] = [];
  let cssAliasRules: CssAliasRule[] = [];
  let extractCss = false;
  // Per-environment define maps: Nuxt shares the same plugin instance for
  // both client and server Vite builds. The server build adds defines like
  // `document: "undefined"` that must NOT leak into client transforms.
  let clientViteDefine: Record<string, string> = {};
  let serverViteDefine: Record<string, string> = {};

  const logger = createLogger(options.debug ?? false);

  async function compileAll(): Promise<void> {
    const startTime = performance.now();
    const files = await glob(scanPatterns, {
      cwd: root,
      ignore: ignorePatterns,
      absolute: true,
    });

    logger.info(`Pre-compiling ${files.length} Vue files...`);

    // Read all files
    const fileContents: { path: string; source: string }[] = [];
    for (const file of files) {
      try {
        const source = fs.readFileSync(file, "utf-8");
        fileContents.push({ path: file, source });
      } catch (e) {
        logger.error(`Failed to read ${file}:`, e);
      }
    }

    // Batch compile using native parallel processing
    const result = compileBatch(fileContents, cache, {
      ssr: mergedOptions.ssr ?? false,
    });

    // Collect CSS for production extraction.
    // Skip files with delegated styles (preprocessor/CSS Modules) -- those go through
    // Vite's CSS pipeline and are extracted by Vite itself.
    if (isProduction) {
      for (const fileResult of result.results) {
        if (fileResult.css) {
          const cached = cache.get(fileResult.path);
          const hasDelegated = cached?.styles?.some(
            (s) =>
              (s.lang !== null && ["scss", "sass", "less", "stylus", "styl"].includes(s.lang)) ||
              s.module !== false,
          );
          if (!hasDelegated) {
            collectedCss.set(
              fileResult.path,
              resolveCssImports(fileResult.css, fileResult.path, cssAliasRules, false),
            );
          }
        }
      }
    }

    const elapsed = (performance.now() - startTime).toFixed(2);
    logger.info(
      `Pre-compilation complete: ${result.successCount} succeeded, ${result.failedCount} failed (${elapsed}ms, native batch: ${result.timeMs.toFixed(2)}ms)`,
    );
  }

  function resolveVuePath(id: string, importer?: string): string {
    let resolved: string;
    // Handle Vite's /@fs/ prefix for absolute filesystem paths
    if (id.startsWith("/@fs/")) {
      resolved = id.slice(4); // Remove '/@fs' prefix, keep the absolute path
    } else if (id.startsWith("/") && !fs.existsSync(id)) {
      // Check if it's a web-root relative path (starts with / but not a real absolute path)
      // These are relative to the project root, not the filesystem root
      // Remove leading slash and resolve relative to root
      resolved = path.resolve(root, id.slice(1));
    } else if (path.isAbsolute(id)) {
      resolved = id;
    } else if (importer) {
      // If importer is a virtual module, extract the real path
      const realImporter = isVizeVirtual(importer) ? fromVirtualId(importer) : importer;
      resolved = path.resolve(path.dirname(realImporter), id);
    } else {
      // Relative path without importer - resolve from root
      resolved = path.resolve(root, id);
    }
    // Ensure we always return an absolute path
    if (!path.isAbsolute(resolved)) {
      resolved = path.resolve(root, resolved);
    }
    return path.normalize(resolved);
  }

  const mainPlugin: Plugin = {
    name: "vite-plugin-vize",
    enforce: "pre",

    config(_, env) {
      return {
        // Vue 3 ESM bundler build requires these compile-time feature flags.
        // @vitejs/plugin-vue normally provides them; vize must do so as its replacement.
        define: {
          __VUE_OPTIONS_API__: true,
          __VUE_PROD_DEVTOOLS__: env.command === "serve",
          __VUE_PROD_HYDRATION_MISMATCH_DETAILS__: false,
        },
        optimizeDeps: {
          exclude: ["virtual:vize-styles"],
        },
      };
    },

    async configResolved(resolvedConfig: ResolvedConfig) {
      root = options.root ?? resolvedConfig.root;
      isProduction = options.isProduction ?? resolvedConfig.isProduction;
      const isSsrBuild = !!resolvedConfig.build?.ssr;
      if (isSsrBuild) {
        serverViteBase = resolvedConfig.base ?? "/";
      } else {
        clientViteBase = resolvedConfig.base ?? "/";
      }
      extractCss = isProduction;

      // Capture custom Vite define values for applying to virtual modules.
      // Vite's built-in define plugin may not process \0-prefixed virtual modules,
      // so we apply replacements ourselves in the transform hook.
      // IMPORTANT: Nuxt shares the same plugin instance for client and server builds,
      // each calling configResolved with environment-specific defines. We must store
      // them separately to avoid the server's `document: "undefined"` leaking into
      // client transforms, or the client's `import.meta.server: false` into server ones.
      const isSsr = !!resolvedConfig.build?.ssr;
      const envDefine: Record<string, string> = {};
      if (resolvedConfig.define) {
        for (const [key, value] of Object.entries(resolvedConfig.define)) {
          if (isBuiltinDefine(key)) continue;
          if (typeof value === "string") {
            envDefine[key] = value;
          } else {
            envDefine[key] = JSON.stringify(value);
          }
        }
      }
      if (isSsr) {
        serverViteDefine = envDefine;
      } else {
        clientViteDefine = envDefine;
      }

      const configEnv: ConfigEnv = {
        mode: resolvedConfig.mode,
        command: resolvedConfig.command === "build" ? "build" : "serve",
        isSsrBuild: !!resolvedConfig.build?.ssr,
      };

      let fileConfig = null;
      if (options.configMode !== false) {
        fileConfig = await loadConfig(root, {
          mode: options.configMode ?? "root",
          configFile: options.configFile,
          env: configEnv,
        });
        if (fileConfig) {
          logger.log("Loaded config from vize.config file");
          vizeConfigStore.set(root, fileConfig);
        }
      }

      const viteConfig = fileConfig?.vite ?? {};
      const compilerConfig = fileConfig?.compiler ?? {};

      mergedOptions = {
        ...options,
        ssr: options.ssr ?? compilerConfig.ssr ?? false,
        sourceMap: options.sourceMap ?? compilerConfig.sourceMap,
        vapor: options.vapor ?? compilerConfig.vapor ?? false,
        include: options.include ?? viteConfig.include,
        exclude: options.exclude ?? viteConfig.exclude,
        scanPatterns: options.scanPatterns ?? viteConfig.scanPatterns,
        ignorePatterns: options.ignorePatterns ?? viteConfig.ignorePatterns,
      };

      dynamicImportAliasRules = [];
      for (const alias of resolvedConfig.resolve.alias) {
        if (typeof alias.find !== "string" || typeof alias.replacement !== "string") {
          continue;
        }
        const fromPrefix = alias.find.endsWith("/") ? alias.find : `${alias.find}/`;
        const replacement = toBrowserImportPrefix(alias.replacement);
        const toPrefix = replacement.endsWith("/") ? replacement : `${replacement}/`;
        dynamicImportAliasRules.push({ fromPrefix, toPrefix });
      }
      // Prefer longer alias keys first (e.g. "@@" before "@")
      dynamicImportAliasRules.sort((a, b) => b.fromPrefix.length - a.fromPrefix.length);

      // Build CSS alias rules for @import resolution (use filesystem paths, not browser paths)
      cssAliasRules = [];
      for (const alias of resolvedConfig.resolve.alias) {
        if (typeof alias.find !== "string" || typeof alias.replacement !== "string") {
          continue;
        }
        cssAliasRules.push({ find: alias.find, replacement: alias.replacement });
      }
      // Prefer longer alias keys first
      cssAliasRules.sort((a, b) => b.find.length - a.find.length);

      filter = createFilter(mergedOptions.include, mergedOptions.exclude);
      scanPatterns = mergedOptions.scanPatterns ?? ["**/*.vue"];
      ignorePatterns = mergedOptions.ignorePatterns ?? ["node_modules/**", "dist/**", ".git/**"];
      initialized = true;
    },

    configureServer(devServer: ViteDevServer) {
      server = devServer;

      // Rewrite __x00__ URLs from virtual module dynamic imports.
      // When compiled .vue files contain dynamic imports (e.g., template literal imports
      // for SVGs), the browser resolves them relative to the virtual module URL which
      // contains \0 (encoded as __x00__). Vite's plugin container short-circuits
      // resolveId for \0-prefixed IDs, so we intercept at the middleware level and
      // rewrite to /@fs/ so Vite serves the actual file.
      devServer.middlewares.use((req, _res, next) => {
        if (req.url && req.url.includes("__x00__")) {
          const [urlPath, queryPart] = req.url.split("?");
          // e.g., /@id/__x00__/Users/.../help.svg?import -> /@fs/Users/.../help.svg?import
          let cleanedPath = urlPath.replace(/__x00__/g, "");
          // After removing __x00__, /@id//Users/... has double slash -- normalize to /@fs/
          cleanedPath = cleanedPath.replace(/^\/@id\/\//, "/@fs/");

          // Do not rewrite vize virtual Vue modules (e.g. /@id/__x00__/.../App.vue.ts),
          // they must go through plugin load() and are not real files on disk.
          if (cleanedPath.startsWith("/@fs/")) {
            const fsPath = cleanedPath.slice(4); // strip '/@fs'
            if (
              fsPath.startsWith("/") &&
              fs.existsSync(fsPath) &&
              fs.statSync(fsPath).isFile() &&
              !fsPath.endsWith(".vue.ts")
            ) {
              const cleaned = queryPart ? `${cleanedPath}?${queryPart}` : cleanedPath;
              if (cleaned !== req.url) {
                logger.log(`middleware: rewriting ${req.url} -> ${cleaned}`);
                req.url = cleaned;
              }
            }
          }
        }
        next();
      });
    },

    async buildStart() {
      if (!scanPatterns) {
        // Running in standalone rolldown context (e.g., ox-content OG image)
        // where configResolved is not called. Skip pre-compilation.
        return;
      }
      await compileAll();
      logger.log("Cache keys:", [...cache.keys()].slice(0, 3));
    },

    async resolveId(id: string, importer?: string) {
      const isBuild = server === null;

      // Skip all virtual module IDs
      if (id.startsWith("\0")) {
        // This is one of our .vue.ts virtual modules -- pass through
        if (isVizeVirtual(id)) {
          return null;
        }
        // Legacy: handle old \0vize: prefixed non-vue files
        if (id.startsWith(LEGACY_VIZE_PREFIX)) {
          const rawPath = id.slice(LEGACY_VIZE_PREFIX.length);
          const cleanPath = rawPath.endsWith(".ts") ? rawPath.slice(0, -3) : rawPath;
          if (!cleanPath.endsWith(".vue")) {
            logger.log(`resolveId: redirecting legacy virtual ID to ${cleanPath}`);
            return cleanPath;
          }
        }
        // Redirect non-vue files that accidentally got \0 prefix.
        // This happens when Vite's import analysis resolves dynamic imports
        // relative to virtual module paths -- the \0 prefix leaks into the
        // resolved path and appears as __x00__ in browser URLs.
        const cleanPath = id.slice(1); // strip \0
        if (cleanPath.startsWith("/") && !cleanPath.endsWith(".vue.ts")) {
          // Strip query params for existence check
          const [pathPart, queryPart] = cleanPath.split("?");
          const querySuffix = queryPart ? `?${queryPart}` : "";
          logger.log(`resolveId: redirecting \0-prefixed non-vue ID to ${pathPart}${querySuffix}`);
          const redirected = pathPart + querySuffix;
          return isBuild ? normalizeFsIdForBuild(redirected) : redirected;
        }
        return null;
      }

      // Handle stale vize: prefix (without \0) from cached resolutions
      if (id.startsWith("vize:")) {
        let realPath = id.slice("vize:".length);
        if (realPath.endsWith(".ts")) {
          realPath = realPath.slice(0, -3);
        }
        logger.log(`resolveId: redirecting stale vize: ID to ${realPath}`);
        const resolved = await this.resolve(realPath, importer, { skipSelf: true });
        if (resolved && isBuild && resolved.id.startsWith("/@fs/")) {
          return { ...resolved, id: normalizeFsIdForBuild(resolved.id) };
        }
        return resolved;
      }

      // Handle virtual CSS module for production extraction
      if (id === VIRTUAL_CSS_MODULE) {
        return RESOLVED_CSS_MODULE;
      }

      if (isBuild && id.startsWith("/@fs/")) {
        return normalizeFsIdForBuild(id);
      }

      // Handle ?macro=true queries (Nuxt page macros: defineRouteRules, definePageMeta, etc.)
      // Nuxt's router generates `import { default } from "page.vue?macro=true"` to extract
      // route metadata. Without @vitejs/plugin-vue, Vize must handle this query and return
      // the compiled script output so Vite's OXC transform can process it as JS.
      if (id.includes("?macro=true")) {
        const filePath = id.split("?")[0];
        const resolved = resolveVuePath(filePath, importer);
        if (resolved && fs.existsSync(resolved)) {
          return `\0${resolved}?macro=true`;
        }
      }

      // Handle virtual style imports:
      //   Component.vue?vue&type=style&index=0&lang=scss
      //   Component.vue?vue&type=style&index=0&lang=scss&module
      if (id.includes("?vue&type=style") || id.includes("?vue=&type=style")) {
        const params = new URLSearchParams(id.split("?")[1]);
        const lang = params.get("lang") || "css";
        if (params.has("module")) {
          // For CSS Modules, append .module.{lang} suffix so Vite's CSS pipeline
          // automatically treats it as a CSS module and returns the class mapping.
          return `\0${id}.module.${lang}`;
        }
        // Append .{lang} suffix so Vite's CSS pipeline recognizes the file type
        // and applies the appropriate preprocessor (SCSS, Less, etc.).
        return `\0${id}.${lang}`;
      }

      // If importer is a vize virtual module or macro module, resolve imports against the real path
      const isMacroImporter = importer?.startsWith("\0") && importer?.endsWith("?macro=true");
      if (importer && (isVizeVirtual(importer) || isMacroImporter)) {
        const cleanImporter = isMacroImporter
          ? importer.slice(1).replace("?macro=true", "")
          : fromVirtualId(importer);

        logger.log(`resolveId from virtual: id=${id}, cleanImporter=${cleanImporter}`);

        // Subpath imports (e.g., #imports/entry from Nuxt)
        if (id.startsWith("#")) {
          try {
            return await this.resolve(id, cleanImporter, { skipSelf: true });
          } catch {
            return null;
          }
        }

        // For non-vue files, resolve relative to the real importer
        if (!id.endsWith(".vue")) {
          // For bare module specifiers (not relative, not absolute),
          // resolve them from the real importer path so that Vite can find
          // packages in the correct node_modules directory.
          if (!id.startsWith("./") && !id.startsWith("../") && !id.startsWith("/")) {
            const matchesAlias = cssAliasRules.some(
              (rule) => id === rule.find || id.startsWith(rule.find + "/"),
            );
            if (!matchesAlias) {
              try {
                const resolved = await this.resolve(id, cleanImporter, { skipSelf: true });
                if (resolved) {
                  logger.log(`resolveId: resolved bare ${id} to ${resolved.id} via Vite resolver`);
                  if (isBuild && resolved.id.startsWith("/@fs/")) {
                    return { ...resolved, id: normalizeFsIdForBuild(resolved.id) };
                  }
                  return resolved;
                }
              } catch {
                // Fall through
              }
            }
            return null;
          }

          // Delegate to Vite's full resolver pipeline with the real importer
          try {
            const resolved = await this.resolve(id, cleanImporter, { skipSelf: true });
            if (resolved) {
              logger.log(`resolveId: resolved ${id} to ${resolved.id} via Vite resolver`);
              if (isBuild && resolved.id.startsWith("/@fs/")) {
                return { ...resolved, id: normalizeFsIdForBuild(resolved.id) };
              }
              return resolved;
            }
          } catch {
            // Fall through to manual resolution
          }

          // Fallback: manual resolution for relative imports
          if (id.startsWith("./") || id.startsWith("../")) {
            const [pathPart, queryPart] = id.split("?");
            const querySuffix = queryPart ? `?${queryPart}` : "";

            const resolved = path.resolve(path.dirname(cleanImporter), pathPart);
            for (const ext of ["", ".ts", ".tsx", ".js", ".jsx", ".json"]) {
              const candidate = resolved + ext;
              if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
                const finalPath = candidate + querySuffix;
                logger.log(`resolveId: resolved relative ${id} to ${finalPath}`);
                return finalPath;
              }
            }
            if (fs.existsSync(resolved) && fs.statSync(resolved).isDirectory()) {
              for (const indexFile of ["/index.ts", "/index.tsx", "/index.js", "/index.jsx"]) {
                const candidate = resolved + indexFile;
                if (fs.existsSync(candidate)) {
                  const finalPath = candidate + querySuffix;
                  logger.log(`resolveId: resolved directory ${id} to ${finalPath}`);
                  return finalPath;
                }
              }
            }
          }

          return null;
        }
      }

      // Handle .vue file imports
      if (id.endsWith(".vue")) {
        const handleNodeModules = initialized ? (mergedOptions.handleNodeModulesVue ?? true) : true;

        if (!handleNodeModules && id.includes("node_modules")) {
          logger.log(`resolveId: skipping node_modules import ${id}`);
          return null;
        }

        const resolved = resolveVuePath(id, importer);
        const isNodeModulesPath = resolved.includes("node_modules");

        if (!handleNodeModules && isNodeModulesPath) {
          logger.log(`resolveId: skipping node_modules path ${resolved}`);
          return null;
        }

        if (filter && !isNodeModulesPath && !filter(resolved)) {
          logger.log(`resolveId: skipping filtered path ${resolved}`);
          return null;
        }

        const hasCache = cache.has(resolved);
        const fileExists = fs.existsSync(resolved);
        logger.log(
          `resolveId: id=${id}, resolved=${resolved}, hasCache=${hasCache}, fileExists=${fileExists}, importer=${importer ?? "none"}`,
        );

        // Return virtual module ID: \0/path/to/Component.vue.ts
        if (hasCache || fileExists) {
          return toVirtualId(resolved);
        }

        // Vite fallback for aliased imports
        if (!fileExists && !path.isAbsolute(id)) {
          const viteResolved = await this.resolve(id, importer, { skipSelf: true });
          if (viteResolved && viteResolved.id.endsWith(".vue")) {
            const realPath = viteResolved.id;
            const isResolvedNodeModules = realPath.includes("node_modules");
            if (
              (isResolvedNodeModules ? handleNodeModules : filter(realPath)) &&
              (cache.has(realPath) || fs.existsSync(realPath))
            ) {
              logger.log(`resolveId: resolved via Vite fallback ${id} to ${realPath}`);
              return toVirtualId(realPath);
            }
          }
        }
      }

      return null;
    },

    load(id: string, loadOptions?: { ssr?: boolean }) {
      // Pick the correct viteBase for URL resolution based on the build environment.
      const currentBase = loadOptions?.ssr ? serverViteBase : clientViteBase;

      // Handle virtual CSS module for production extraction
      if (id === RESOLVED_CSS_MODULE) {
        const allCss = Array.from(collectedCss.values()).join("\n\n");
        return allCss;
      }

      // Strip the \0 prefix and the appended extension suffix for style virtual IDs.
      let styleId = id;
      if (id.startsWith("\0") && id.includes("?vue")) {
        styleId = id
          .slice(1) // strip \0
          .replace(/\.module\.\w+$/, "") // strip .module.{lang}
          .replace(/\.\w+$/, ""); // strip .{lang}
      }

      if (styleId.includes("?vue&type=style") || styleId.includes("?vue=&type=style")) {
        const [filename, queryString] = styleId.split("?");
        const realPath = isVizeVirtual(filename) ? fromVirtualId(filename) : filename;
        const params = new URLSearchParams(queryString);
        const indexStr = params.get("index");
        const lang = params.get("lang");
        const _hasModule = params.has("module");
        const scoped = params.get("scoped");

        const compiled = cache.get(realPath);
        const blockIndex = indexStr !== null ? parseInt(indexStr, 10) : -1;

        if (compiled?.styles && blockIndex >= 0 && blockIndex < compiled.styles.length) {
          const block = compiled.styles[blockIndex];
          let styleContent = block.content;

          // For scoped preprocessor styles, wrap content in a scope selector
          if (scoped && block.scoped && lang && lang !== "css") {
            const lines = styleContent.split("\n");
            const hoisted: string[] = [];
            const body: string[] = [];
            for (const line of lines) {
              const trimmed = line.trimStart();
              if (
                trimmed.startsWith("@use ") ||
                trimmed.startsWith("@forward ") ||
                trimmed.startsWith("@import ")
              ) {
                hoisted.push(line);
              } else {
                body.push(line);
              }
            }
            const bodyContent = body.join("\n");
            const hoistedContent = hoisted.length > 0 ? hoisted.join("\n") + "\n\n" : "";
            styleContent = `${hoistedContent}[${scoped}] {\n${bodyContent}\n}`;
          }

          return {
            code: styleContent,
            map: null,
          };
        }

        if (compiled?.css) {
          return resolveCssImports(
            compiled.css,
            realPath,
            cssAliasRules,
            server !== null,
            currentBase,
          );
        }
        return "";
      }

      // Handle ?macro=true queries
      if (id.startsWith("\0") && id.endsWith("?macro=true")) {
        const realPath = id.slice(1).replace("?macro=true", "");
        if (fs.existsSync(realPath)) {
          const source = fs.readFileSync(realPath, "utf-8");
          const setupMatch = source.match(/<script\s+setup[^>]*>([\s\S]*?)<\/script>/);
          if (setupMatch) {
            const scriptContent = setupMatch[1];
            return {
              code: `${scriptContent}\nexport default {}`,
              map: null,
            };
          }
        }
        return { code: "export default {}", map: null };
      }

      // Handle vize virtual modules
      if (isVizeVirtual(id)) {
        const realPath = fromVirtualId(id);

        if (!realPath.endsWith(".vue")) {
          logger.log(`load: skipping non-vue virtual module ${realPath}`);
          return null;
        }

        let compiled = cache.get(realPath);

        // On-demand compile if not cached
        if (!compiled && fs.existsSync(realPath)) {
          logger.log(`load: on-demand compiling ${realPath}`);
          compiled = compileFile(realPath, cache, {
            sourceMap: mergedOptions?.sourceMap ?? !(isProduction ?? false),
            ssr: mergedOptions?.ssr ?? false,
          });
        }

        if (compiled) {
          const hasDelegated = compiled.styles?.some(
            (s) =>
              (s.lang !== null && ["scss", "sass", "less", "stylus", "styl"].includes(s.lang)) ||
              s.module !== false,
          );
          if (compiled.css && !hasDelegated) {
            compiled = {
              ...compiled,
              css: resolveCssImports(
                compiled.css,
                realPath,
                cssAliasRules,
                server !== null,
                currentBase,
              ),
            };
          }
          const output = rewriteStaticAssetUrls(
            rewriteDynamicTemplateImports(
              generateOutput(compiled, {
                isProduction,
                isDev: server !== null,
                extractCss,
                filePath: realPath,
              }),
              dynamicImportAliasRules,
            ),
            dynamicImportAliasRules,
          );
          return {
            code: output,
            map: null,
          };
        }
      }

      // Handle \0-prefixed non-vue files leaked from virtual module dynamic imports.
      if (id.startsWith("\0")) {
        const afterPrefix = id.startsWith(LEGACY_VIZE_PREFIX)
          ? id.slice(LEGACY_VIZE_PREFIX.length)
          : id.slice(1);
        if (afterPrefix.includes("?commonjs-")) {
          return null;
        }
        const [pathPart, queryPart] = afterPrefix.split("?");
        const querySuffix = queryPart ? `?${queryPart}` : "";
        const fsPath = pathPart.startsWith("/@fs/") ? pathPart.slice(4) : pathPart;
        if (fsPath.startsWith("/") && fs.existsSync(fsPath) && fs.statSync(fsPath).isFile()) {
          const importPath =
            server === null
              ? `${pathToFileURL(fsPath).href}${querySuffix}`
              : "/@fs" + fsPath + querySuffix;
          logger.log(`load: proxying \0-prefixed file ${id} -> re-export from ${importPath}`);
          return `export { default } from ${JSON.stringify(importPath)};\nexport * from ${JSON.stringify(importPath)};`;
        }
      }

      return null;
    },

    // Strip TypeScript from compiled .vue output and apply define replacements
    async transform(
      code: string,
      id: string,
      options?: { ssr?: boolean },
    ): Promise<TransformResult | null> {
      const isMacro = id.startsWith("\0") && id.endsWith("?macro=true");
      if (isVizeVirtual(id) || isMacro) {
        const realPath = isMacro ? id.slice(1).replace("?macro=true", "") : fromVirtualId(id);
        try {
          const result = await transformWithOxc(code, realPath, {
            lang: "ts",
          });
          const defines = options?.ssr ? serverViteDefine : clientViteDefine;
          let transformed = result.code;
          if (Object.keys(defines).length > 0) {
            transformed = applyDefineReplacements(transformed, defines);
          }

          return { code: transformed, map: result.map as TransformResult["map"] };
        } catch (e: unknown) {
          logger.error(`transformWithOxc failed for ${realPath}:`, e);
          const dumpPath = `/tmp/vize-oxc-error-${path.basename(realPath)}.ts`;
          fs.writeFileSync(dumpPath, code, "utf-8");
          logger.error(`Dumped failing code to ${dumpPath}`);
          return { code: "export default {}", map: null };
        }
      }

      return null;
    },

    async handleHotUpdate(ctx: HmrContext) {
      const { file, server, read } = ctx;

      if (file.endsWith(".vue") && filter(file)) {
        try {
          const source = await read();

          const prevCompiled = cache.get(file);

          compileFile(
            file,
            cache,
            {
              sourceMap: mergedOptions?.sourceMap ?? !isProduction,
              ssr: mergedOptions?.ssr ?? false,
            },
            source,
          );

          const newCompiled = cache.get(file)!;

          const updateType: HmrUpdateType = detectHmrUpdateType(prevCompiled, newCompiled);

          logger.log(`Re-compiled: ${path.relative(root, file)} (${updateType})`);

          const virtualId = toVirtualId(file);
          const modules =
            server.moduleGraph.getModulesByFile(virtualId) ??
            server.moduleGraph.getModulesByFile(file);

          const hasDelegated = newCompiled.styles?.some(
            (s) =>
              (s.lang !== null && ["scss", "sass", "less", "stylus", "styl"].includes(s.lang)) ||
              s.module !== false,
          );

          if (hasDelegated && updateType === "style-only") {
            const affectedModules: Set<import("vite").ModuleNode> = new Set();
            for (const block of newCompiled.styles ?? []) {
              const params = new URLSearchParams();
              params.set("vue", "");
              params.set("type", "style");
              params.set("index", String(block.index));
              if (block.scoped) params.set("scoped", `data-v-${newCompiled.scopeId}`);
              params.set("lang", block.lang ?? "css");
              if (block.module !== false) {
                params.set("module", typeof block.module === "string" ? block.module : "");
              }
              const styleId = `${file}?${params.toString()}`;
              const styleMods = server.moduleGraph.getModulesByFile(styleId);
              if (styleMods) {
                for (const mod of styleMods) {
                  affectedModules.add(mod);
                }
              }
            }
            if (modules) {
              for (const mod of modules) {
                affectedModules.add(mod);
              }
            }
            if (affectedModules.size > 0) {
              return [...affectedModules];
            }
          }

          if (updateType === "style-only" && newCompiled.css && !hasDelegated) {
            server.ws.send({
              type: "custom",
              event: "vize:update",
              data: {
                id: newCompiled.scopeId,
                type: "style-only",
                css: resolveCssImports(newCompiled.css, file, cssAliasRules, true, clientViteBase),
              },
            });
            return [];
          }

          if (modules) {
            return [...modules];
          }
        } catch (e) {
          logger.error(`Re-compilation failed for ${file}:`, e);
        }
      }
    },

    generateBundle(_, _bundle) {
      if (!extractCss || collectedCss.size === 0) {
        return;
      }

      const allCss = Array.from(collectedCss.values()).join("\n\n");
      if (allCss.trim()) {
        this.emitFile({
          type: "asset",
          fileName: "assets/vize-components.css",
          source: allCss,
        });
        logger.log(`Extracted CSS to assets/vize-components.css (${collectedCss.size} components)`);
      }
    },
  };

  let compilerSfc: unknown = null;
  const loadCompilerSfc = () => {
    if (!compilerSfc) {
      try {
        const require = createRequire(import.meta.url);
        compilerSfc = require("@vue/compiler-sfc");
      } catch {
        compilerSfc = { parse: () => ({ descriptor: {}, errors: [] }) };
      }
    }
    return compilerSfc;
  };
  const vueCompatPlugin: Plugin = {
    name: "vite:vue",
    api: {
      get options() {
        return {
          compiler: loadCompilerSfc(),
          isProduction: isProduction ?? false,
          root: root ?? process.cwd(),
          template: {},
        };
      },
    },
  };

  // Post-transform plugin to handle virtual SFC content from other plugins.
  const postTransformPlugin: Plugin = {
    name: "vize:post-transform",
    enforce: "post",
    async transform(
      code: string,
      id: string,
      transformOptions?: { ssr?: boolean },
    ): Promise<TransformResult | null> {
      if (
        !id.endsWith(".vue") &&
        !id.endsWith(".vue.ts") &&
        !id.includes("node_modules") &&
        id.endsWith(".setup.ts") &&
        /<script\s+setup[\s>]/.test(code)
      ) {
        logger.log(`post-transform: compiling virtual SFC content from ${id}`);
        try {
          const compiled = compileFile(
            id,
            cache,
            {
              sourceMap: mergedOptions?.sourceMap ?? !(isProduction ?? false),
              ssr: mergedOptions?.ssr ?? false,
            },
            code,
          );

          const output = generateOutput(compiled, {
            isProduction,
            isDev: server !== null,
            extractCss,
            filePath: id,
          });

          const result = await transformWithOxc(output, id, { lang: "ts" });
          const defines = transformOptions?.ssr ? serverViteDefine : clientViteDefine;
          let transformed = result.code;
          if (Object.keys(defines).length > 0) {
            transformed = applyDefineReplacements(transformed, defines);
          }
          return { code: transformed, map: result.map as TransformResult["map"] };
        } catch (e: unknown) {
          logger.error(`Virtual SFC compilation failed for ${id}:`, e);
        }
      }
      return null;
    },
  };

  return [vueCompatPlugin, mainPlugin, postTransformPlugin];
}
