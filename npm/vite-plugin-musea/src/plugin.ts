/**
 * Main Musea Vite plugin implementation.
 *
 * Contains the `musea()` factory function that creates the Vite plugin,
 * including dev server middleware, virtual module handling, and HMR support.
 *
 * Middleware and API route logic is extracted into:
 * - `server-middleware.ts` -- gallery SPA, preview, art module serving
 * - `api-routes.ts` -- REST API endpoints for gallery UI
 */

import type { Plugin, ViteDevServer, ResolvedConfig } from "vite";
import fs from "node:fs";
import path from "node:path";
import { vizeConfigStore } from "@vizejs/vite-plugin";

import type { MuseaOptions, ArtFileInfo } from "./types.js";

import { loadNative } from "./native-loader.js";
import { generateGalleryModule } from "./gallery.js";
import { generatePreviewModule } from "./preview.js";
import { generateManifestModule } from "./manifest.js";
import { generateArtModule, extractScriptSetupContent } from "./art-module.js";
import {
  shouldProcess,
  scanArtFiles,
  generateStorybookFiles,
  toPascalCase,
  buildThemeConfig,
} from "./utils.js";
import { registerMiddleware } from "./server-middleware.js";
import { createApiMiddleware } from "./api-routes.js";

// Virtual module prefixes
const VIRTUAL_MUSEA_PREFIX = "\0musea:";
const VIRTUAL_GALLERY = "\0musea-gallery";
const VIRTUAL_MANIFEST = "\0musea-manifest";

/**
 * Create Musea Vite plugin.
 */
export function musea(options: MuseaOptions = {}): Plugin[] {
  let include = options.include ?? ["**/*.art.vue"];
  let exclude = options.exclude ?? ["node_modules/**", "dist/**"];
  let basePath = options.basePath ?? "/__musea__";
  let storybookCompat = options.storybookCompat ?? false;
  const storybookOutDir = options.storybookOutDir ?? ".storybook/stories";
  let inlineArt = options.inlineArt ?? false;
  const tokensPath = options.tokensPath;
  const themeConfig = buildThemeConfig(options.theme);
  const previewCss = options.previewCss ?? [];
  const previewSetup = options.previewSetup;

  let config: ResolvedConfig;
  let server: ViteDevServer | null = null;
  const artFiles = new Map<string, ArtFileInfo>();
  let resolvedPreviewCss: string[] = [];
  let resolvedPreviewSetup: string | null = null;

  // Main plugin
  const mainPlugin: Plugin = {
    name: "vite-plugin-musea",
    enforce: "pre",

    config() {
      // Add Vue alias for runtime template compilation
      // This is needed because variant templates are compiled at runtime
      return {
        resolve: {
          alias: {
            vue: "vue/dist/vue.esm-bundler.js",
          },
        },
      };
    },

    configResolved(resolvedConfig) {
      config = resolvedConfig;

      // Merge musea config from vize.config.ts (plugin args > config file > defaults)
      const vizeConfig = vizeConfigStore.get(resolvedConfig.root);
      if (vizeConfig?.musea) {
        const mc = vizeConfig.musea;
        // Only apply config file values when plugin options were not explicitly set
        if (!options.include && mc.include) include = mc.include;
        if (!options.exclude && mc.exclude) exclude = mc.exclude;
        if (!options.basePath && mc.basePath) basePath = mc.basePath;
        if (options.storybookCompat === undefined && mc.storybookCompat !== undefined)
          storybookCompat = mc.storybookCompat;
        if (options.inlineArt === undefined && mc.inlineArt !== undefined) inlineArt = mc.inlineArt;
      }

      // Resolve previewCss paths to absolute paths
      resolvedPreviewCss = previewCss.map((cssPath) =>
        path.isAbsolute(cssPath) ? cssPath : path.resolve(resolvedConfig.root, cssPath),
      );

      // Resolve previewSetup path
      if (previewSetup) {
        resolvedPreviewSetup = path.isAbsolute(previewSetup)
          ? previewSetup
          : path.resolve(resolvedConfig.root, previewSetup);
      }
    },

    configureServer(devServer) {
      server = devServer;

      // Register gallery SPA, preview, and art module middleware
      registerMiddleware(devServer, {
        basePath,
        themeConfig,
        artFiles,
        resolvedPreviewCss,
        resolvedPreviewSetup,
      });

      // Register API endpoints
      devServer.middlewares.use(
        `${basePath}/api`,
        createApiMiddleware({
          config,
          artFiles,
          tokensPath,
          basePath,
          resolvedPreviewCss,
          resolvedPreviewSetup,
          processArtFile,
          getDevServerPort: () => devServer.config.server.port || 5173,
        }),
      );

      // Watch for Art file changes
      devServer.watcher.on("change", async (file) => {
        if (file.endsWith(".art.vue") && shouldProcess(file, include, exclude, config.root)) {
          await processArtFile(file);
          console.log(`[musea] Reloaded: ${path.relative(config.root, file)}`);
        }
        // Inline art: re-check .vue files on change
        if (inlineArt && file.endsWith(".vue") && !file.endsWith(".art.vue")) {
          const hadArt = artFiles.has(file);
          const source = await fs.promises.readFile(file, "utf-8");
          if (source.includes("<art")) {
            await processArtFile(file);
            console.log(`[musea] Reloaded inline art: ${path.relative(config.root, file)}`);
          } else if (hadArt) {
            artFiles.delete(file);
            console.log(`[musea] Removed inline art: ${path.relative(config.root, file)}`);
          }
        }
      });

      devServer.watcher.on("add", async (file) => {
        if (file.endsWith(".art.vue") && shouldProcess(file, include, exclude, config.root)) {
          await processArtFile(file);
          console.log(`[musea] Added: ${path.relative(config.root, file)}`);
        }
        // Inline art: check new .vue files
        if (inlineArt && file.endsWith(".vue") && !file.endsWith(".art.vue")) {
          const source = await fs.promises.readFile(file, "utf-8");
          if (source.includes("<art")) {
            await processArtFile(file);
            console.log(`[musea] Added inline art: ${path.relative(config.root, file)}`);
          }
        }
      });

      devServer.watcher.on("unlink", (file) => {
        if (artFiles.has(file)) {
          artFiles.delete(file);
          console.log(`[musea] Removed: ${path.relative(config.root, file)}`);
        }
      });

      // Print Musea gallery URL after server starts
      return () => {
        devServer.httpServer?.once("listening", () => {
          const address = devServer.httpServer?.address();
          if (address && typeof address === "object") {
            const protocol = devServer.config.server.https ? "https" : "http";
            const rawHost = address.address;
            // Normalize IPv6/IPv4 localhost addresses to "localhost"
            const host =
              rawHost === "::" ||
              rawHost === "::1" ||
              rawHost === "0.0.0.0" ||
              rawHost === "127.0.0.1"
                ? "localhost"
                : rawHost;
            const port = address.port;
            const url = `${protocol}://${host}:${port}${basePath}`;

            console.log();
            console.log(`  \x1b[36m➜\x1b[0m  \x1b[1mMusea Gallery:\x1b[0m \x1b[36m${url}\x1b[0m`);
          }
        });
      };
    },

    async buildStart() {
      // Scan for Art files
      console.log(`[musea] config.root: ${config.root}, include: ${JSON.stringify(include)}`);
      const files = await scanArtFiles(config.root, include, exclude, inlineArt);

      console.log(`[musea] Found ${files.length} art files`);

      for (const file of files) {
        await processArtFile(file);
      }

      // Generate Storybook CSF if enabled
      if (storybookCompat) {
        await generateStorybookFiles(artFiles, config.root, storybookOutDir);
      }
    },

    resolveId(id) {
      if (id === VIRTUAL_GALLERY) {
        return VIRTUAL_GALLERY;
      }
      if (id === VIRTUAL_MANIFEST) {
        return VIRTUAL_MANIFEST;
      }
      // Handle virtual:musea-preview: prefix for preview modules
      if (id.startsWith("virtual:musea-preview:")) {
        return "\0musea-preview:" + id.slice("virtual:musea-preview:".length);
      }
      // Handle virtual:musea-art: prefix for preview modules
      // Append ?musea-virtual to prevent other plugins (e.g. unplugin-vue-i18n)
      // from treating .vue-ending virtual IDs as Vue SFC files
      if (id.startsWith("virtual:musea-art:")) {
        const artPath = id.slice("virtual:musea-art:".length);
        if (artFiles.has(artPath)) {
          return "\0musea-art:" + artPath + "?musea-virtual";
        }
      }
      if (id.endsWith(".art.vue")) {
        const resolved = path.resolve(config.root, id);
        if (artFiles.has(resolved)) {
          return VIRTUAL_MUSEA_PREFIX + resolved + "?musea-virtual";
        }
      }
      // Inline art: resolve .vue files that have <art> blocks
      if (inlineArt && id.endsWith(".vue") && !id.endsWith(".art.vue")) {
        const resolved = path.resolve(config.root, id);
        if (artFiles.has(resolved)) {
          return VIRTUAL_MUSEA_PREFIX + resolved + "?musea-virtual";
        }
      }
      return null;
    },

    load(id) {
      if (id === VIRTUAL_GALLERY) {
        return generateGalleryModule(basePath);
      }
      if (id === VIRTUAL_MANIFEST) {
        return generateManifestModule(artFiles);
      }
      // Handle \0musea-preview: prefix for preview modules
      if (id.startsWith("\0musea-preview:")) {
        const rest = id.slice("\0musea-preview:".length);
        const lastColonIndex = rest.lastIndexOf(":");
        if (lastColonIndex !== -1) {
          const artPath = rest.slice(0, lastColonIndex);
          const variantName = rest.slice(lastColonIndex + 1);
          const art = artFiles.get(artPath);
          if (art) {
            const variantComponentName = toPascalCase(variantName);
            return generatePreviewModule(
              art,
              variantComponentName,
              variantName,
              resolvedPreviewCss,
              resolvedPreviewSetup,
            );
          }
        }
      }
      // Handle \0musea-art: prefix for preview modules
      if (id.startsWith("\0musea-art:")) {
        const artPath = id.slice("\0musea-art:".length).replace(/\?musea-virtual$/, "");
        const art = artFiles.get(artPath);
        if (art) {
          return generateArtModule(art, artPath);
        }
      }
      if (id.startsWith(VIRTUAL_MUSEA_PREFIX)) {
        const realPath = id.slice(VIRTUAL_MUSEA_PREFIX.length).replace(/\?musea-virtual$/, "");
        const art = artFiles.get(realPath);
        if (art) {
          return generateArtModule(art, realPath);
        }
      }
      return null;
    },

    async handleHotUpdate(ctx) {
      const { file } = ctx;
      if (file.endsWith(".art.vue") && artFiles.has(file)) {
        await processArtFile(file);

        // Invalidate virtual modules
        const virtualId = VIRTUAL_MUSEA_PREFIX + file + "?musea-virtual";
        const modules = server?.moduleGraph.getModulesByFile(virtualId);
        if (modules) {
          return [...modules];
        }
      }

      // Inline art: HMR for .vue files with <art> blocks
      if (inlineArt && file.endsWith(".vue") && !file.endsWith(".art.vue") && artFiles.has(file)) {
        await processArtFile(file);

        const virtualId = VIRTUAL_MUSEA_PREFIX + file;
        const modules = server?.moduleGraph.getModulesByFile(virtualId);
        if (modules) {
          return [...modules];
        }
      }

      return undefined;
    },
  };

  // Helper functions scoped to this plugin instance

  async function processArtFile(filePath: string): Promise<void> {
    try {
      const source = await fs.promises.readFile(filePath, "utf-8");
      const binding = loadNative();
      const parsed = binding.parseArt(source, { filename: filePath });

      // Skip files with no variants (e.g. .vue files without <art> block)
      if (!parsed.variants || parsed.variants.length === 0) return;

      const isInline = !filePath.endsWith(".art.vue");

      const info: ArtFileInfo = {
        path: filePath,
        metadata: {
          title: parsed.metadata.title || (isInline ? path.basename(filePath, ".vue") : ""),
          description: parsed.metadata.description,
          component: isInline ? undefined : parsed.metadata.component,
          category: parsed.metadata.category,
          tags: parsed.metadata.tags,
          status: parsed.metadata.status as "draft" | "ready" | "deprecated",
          order: parsed.metadata.order,
        },
        variants: parsed.variants.map((v) => ({
          name: v.name,
          template: v.template,
          isDefault: v.isDefault,
          skipVrt: v.skipVrt,
        })),
        hasScriptSetup: isInline ? false : parsed.hasScriptSetup,
        scriptSetupContent:
          !isInline && parsed.hasScriptSetup ? extractScriptSetupContent(source) : undefined,
        hasScript: parsed.hasScript,
        styleCount: parsed.styleCount,
        isInline,
        componentPath: isInline ? filePath : undefined,
      };

      artFiles.set(filePath, info);
    } catch (e) {
      console.error(`[musea] Failed to process ${filePath}:`, e);
    }
  }

  return [mainPlugin];
}
