/**
 * Musea gallery API route handlers.
 *
 * Extracted from the main plugin to keep file sizes manageable.
 * Provides REST API endpoints consumed by the gallery UI:
 * - GET/POST/PUT/DELETE /api/tokens  (delegated to api-tokens.ts)
 * - GET /api/arts, /api/arts/:path, /api/arts/:path/source, etc.
 * - POST /api/preview-with-props
 * - POST /api/generate
 * - POST /api/run-vrt
 */

import type { IncomingMessage, ServerResponse } from "node:http";
import type { ResolvedConfig } from "vite";
import fs from "node:fs";
import path from "node:path";

import type { ArtFileInfo } from "./types.js";
import { loadNative, analyzeSfcFallback } from "./native-loader.js";
import { generatePreviewModuleWithProps } from "./preview.js";
import { toPascalCase } from "./utils.js";
import {
  handleTokensUsage,
  handleTokensGet,
  handleTokensCreate,
  handleTokensUpdate,
  handleTokensDelete,
} from "./api-tokens.js";

/** Dependencies injected from the plugin closure. */
export interface ApiRoutesContext {
  config: ResolvedConfig;
  artFiles: Map<string, ArtFileInfo>;
  tokensPath: string | undefined;
  basePath: string;
  resolvedPreviewCss: string[];
  resolvedPreviewSetup: string | null;
  processArtFile: (filePath: string) => Promise<void>;
  /** Reference to the dev server for VRT port resolution */
  getDevServerPort: () => number;
}

export type SendJson = (data: unknown, status?: number) => void;
export type SendError = (message: string, status?: number) => void;
export type ReadBody = () => Promise<string>;

type NextFn = () => void;

/** Helper to read the full request body as a string. */
function collectBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve) => {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => resolve(body));
  });
}

/**
 * Create the API middleware handler for the Musea gallery.
 *
 * Returns a Connect-compatible middleware function that handles all
 * `/api/...` sub-routes under the configured basePath.
 */
export function createApiMiddleware(ctx: ApiRoutesContext) {
  return async (req: IncomingMessage, res: ServerResponse, next: NextFn) => {
    const sendJson: SendJson = (data: unknown, status = 200) => {
      res.statusCode = status;
      res.setHeader("Content-Type", "application/json");
      res.end(JSON.stringify(data));
    };

    const sendError: SendError = (message: string, status = 500) => {
      sendJson({ error: message }, status);
    };

    const readBody: ReadBody = () => collectBody(req);

    const url = req.url || "/";

    // --- GET /api/arts ---
    if (url === "/arts" && req.method === "GET") {
      sendJson(Array.from(ctx.artFiles.values()));
      return;
    }

    // --- Token routes (delegated to api-tokens.ts) ---
    if (url === "/tokens/usage" && req.method === "GET") {
      await handleTokensUsage(ctx, sendJson);
      return;
    }
    if (url === "/tokens" && req.method === "GET") {
      await handleTokensGet(ctx, sendJson);
      return;
    }
    if (url === "/tokens" && req.method === "POST") {
      await handleTokensCreate(ctx, readBody, sendJson, sendError);
      return;
    }
    if (url === "/tokens" && req.method === "PUT") {
      await handleTokensUpdate(ctx, readBody, sendJson, sendError);
      return;
    }
    if (url === "/tokens" && req.method === "DELETE") {
      await handleTokensDelete(ctx, readBody, sendJson, sendError);
      return;
    }

    // --- PUT /api/arts/:path/source (update art source) ---
    if (url?.startsWith("/arts/") && req.method === "PUT") {
      const rest = url.slice(6);
      const sourceMatch = rest.match(/^(.+)\/source$/);
      if (sourceMatch) {
        const artPath = decodeURIComponent(sourceMatch[1]);
        const art = ctx.artFiles.get(artPath);
        if (!art) {
          sendError("Art not found", 404);
          return;
        }

        let body = "";
        req.on("data", (chunk: string) => {
          body += chunk;
        });
        req.on("end", async () => {
          try {
            const { source } = JSON.parse(body) as { source: string };
            if (typeof source !== "string") {
              sendError("Missing required field: source", 400);
              return;
            }
            await fs.promises.writeFile(artPath, source, "utf-8");
            await ctx.processArtFile(artPath);
            sendJson({ success: true });
          } catch (e) {
            sendError(e instanceof Error ? e.message : String(e));
          }
        });
        return;
      }
      next();
      return;
    }

    // --- GET /api/arts/:path/... sub-routes ---
    if (url?.startsWith("/arts/") && req.method === "GET") {
      const rest = url.slice(6);

      const sourceMatch = rest.match(/^(.+)\/source$/);
      const paletteMatch = rest.match(/^(.+)\/palette$/);
      const analysisMatch = rest.match(/^(.+)\/analysis$/);
      const docsMatch = rest.match(/^(.+)\/docs$/);
      const a11yMatch = rest.match(/^(.+)\/variants\/([^/]+)\/a11y$/);

      if (sourceMatch) {
        await handleArtSource(ctx, sourceMatch, sendJson, sendError);
        return;
      }

      if (paletteMatch) {
        await handleArtPalette(ctx, paletteMatch, sendJson, sendError);
        return;
      }

      if (analysisMatch) {
        await handleArtAnalysis(ctx, analysisMatch, sendJson, sendError);
        return;
      }

      if (docsMatch) {
        await handleArtDocs(ctx, docsMatch, sendJson, sendError);
        return;
      }

      if (a11yMatch) {
        handleArtA11y(ctx, a11yMatch, sendJson, sendError);
        return;
      }

      // GET /api/arts/:path (no sub-resource)
      const artPath = decodeURIComponent(rest);
      const art = ctx.artFiles.get(artPath);
      if (art) {
        sendJson(art);
      } else {
        sendError("Art not found", 404);
      }
      return;
    }

    // --- POST /api/preview-with-props ---
    if (url === "/preview-with-props" && req.method === "POST") {
      let body = "";
      req.on("data", (chunk) => {
        body += chunk;
      });
      req.on("end", () => {
        try {
          const { artPath: reqArtPath, variantName, props: propsOverride } = JSON.parse(body);
          const art = ctx.artFiles.get(reqArtPath);
          if (!art) {
            sendError("Art not found", 404);
            return;
          }

          const variant = art.variants.find((v) => v.name === variantName);
          if (!variant) {
            sendError("Variant not found", 404);
            return;
          }

          const variantComponentName = toPascalCase(variant.name);
          const moduleCode = generatePreviewModuleWithProps(
            art,
            variantComponentName,
            variant.name,
            propsOverride,
            ctx.resolvedPreviewCss,
            ctx.resolvedPreviewSetup,
          );
          res.setHeader("Content-Type", "application/javascript");
          res.end(moduleCode);
        } catch (e) {
          sendError(e instanceof Error ? e.message : String(e));
        }
      });
      return;
    }

    // --- POST /api/generate ---
    if (url === "/generate" && req.method === "POST") {
      let body = "";
      req.on("data", (chunk) => {
        body += chunk;
      });
      req.on("end", async () => {
        try {
          const { componentPath: reqComponentPath, options: autogenOptions } = JSON.parse(body);
          const { generateArtFile: genArt } = await import("./autogen.js");
          const result = await genArt(reqComponentPath, autogenOptions);
          sendJson({
            generated: true,
            componentName: result.componentName,
            variants: result.variants,
            artFileContent: result.artFileContent,
          });
        } catch (e) {
          sendError(e instanceof Error ? e.message : String(e));
        }
      });
      return;
    }

    // --- POST /api/run-vrt ---
    if (url === "/run-vrt" && req.method === "POST") {
      let body = "";
      req.on("data", (chunk) => {
        body += chunk;
      });
      req.on("end", async () => {
        try {
          const { artPath, updateSnapshots } = JSON.parse(body);
          const { MuseaVrtRunner } = await import("./vrt.js");

          const runner = new MuseaVrtRunner({
            snapshotDir: path.resolve(ctx.config.root, ".vize/snapshots"),
          });

          const port = ctx.getDevServerPort();
          const baseUrl = `http://localhost:${port}`;

          let artsToTest = Array.from(ctx.artFiles.values());
          if (artPath) {
            artsToTest = artsToTest.filter((a) => a.path === artPath);
          }

          await runner.start();
          const results = await runner.runTests(artsToTest, baseUrl, {
            updateSnapshots,
          });
          const summary = runner.getSummary(results);
          await runner.stop();

          sendJson({
            success: true,
            summary,
            results: results.map((r) => ({
              artPath: r.artPath,
              variantName: r.variantName,
              viewport: r.viewport.name,
              passed: r.passed,
              isNew: r.isNew,
              diffPercentage: r.diffPercentage,
              error: r.error,
            })),
          });
        } catch (e) {
          sendError(e instanceof Error ? e.message : String(e));
        }
      });
      return;
    }

    next();
  };
}

// ---------------------------------------------------------------------------
// Sub-route handlers for GET /api/arts/:path/...
// ---------------------------------------------------------------------------

/** GET /api/arts/:path/source */
async function handleArtSource(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): Promise<void> {
  const artPath = decodeURIComponent(match[1]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  try {
    const source = await fs.promises.readFile(artPath, "utf-8");
    sendJson({ source, path: artPath });
  } catch (e) {
    sendError(e instanceof Error ? e.message : String(e));
  }
}

/** GET /api/arts/:path/palette */
async function handleArtPalette(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): Promise<void> {
  const artPath = decodeURIComponent(match[1]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  try {
    const source = await fs.promises.readFile(artPath, "utf-8");
    const binding = loadNative();
    let palette: {
      title: string;
      controls: Array<{
        name: string;
        control: string;
        default_value?: unknown;
        description?: string;
        required: boolean;
        options: Array<{ label: string; value: unknown }>;
        range?: { min: number; max: number; step?: number };
        group?: string;
      }>;
      groups: string[];
      json: string;
      typescript: string;
    };
    if (binding.generateArtPalette) {
      palette = binding.generateArtPalette(source, {
        filename: artPath,
      });
    } else {
      palette = {
        title: art.metadata.title,
        controls: [],
        groups: [],
        json: "{}",
        typescript: "",
      };
    }

    // If the native palette returned no controls, try JS-based SFC analysis
    if (palette.controls.length === 0 && art.metadata.component) {
      const resolvedComponentPath = path.isAbsolute(art.metadata.component)
        ? art.metadata.component
        : path.resolve(path.dirname(artPath), art.metadata.component);
      try {
        const componentSource = await fs.promises.readFile(resolvedComponentPath, "utf-8");
        const analysis = binding.analyzeSfc
          ? binding.analyzeSfc(componentSource, {
              filename: resolvedComponentPath,
            })
          : analyzeSfcFallback(componentSource, {
              filename: resolvedComponentPath,
            });

        if (analysis.props.length > 0) {
          palette.controls = analysis.props.map((prop) => {
            let control = "text";
            if (prop.type === "boolean") control = "boolean";
            else if (prop.type === "number") control = "number";
            else if (prop.type.includes("|") && !prop.type.includes("=>")) {
              control = "select";
            }

            const options: Array<{ label: string; value: unknown }> = [];
            if (control === "select") {
              const optionMatches = prop.type.match(/"([^"]+)"/g);
              if (optionMatches) {
                for (const opt of optionMatches) {
                  const val = opt.replace(/"/g, "");
                  options.push({ label: val, value: val });
                }
              }
            }

            return {
              name: prop.name,
              control,
              default_value:
                prop.default_value !== undefined
                  ? prop.default_value === "true"
                    ? true
                    : prop.default_value === "false"
                      ? false
                      : typeof prop.default_value === "string" && prop.default_value.startsWith('"')
                        ? prop.default_value.replace(/^"|"$/g, "")
                        : prop.default_value
                  : undefined,
              description: undefined,
              required: prop.required,
              options,
              range: undefined,
              group: undefined,
            };
          });

          palette.json = JSON.stringify(
            { title: palette.title, controls: palette.controls },
            null,
            2,
          );
          palette.typescript = `export interface ${palette.title}Props {\n${palette.controls
            .map(
              (c) =>
                `  ${c.name}${c.required ? "" : "?"}: ${
                  c.control === "boolean"
                    ? "boolean"
                    : c.control === "number"
                      ? "number"
                      : c.control === "select"
                        ? c.options.map((o) => `"${String(o.value)}"`).join(" | ")
                        : "string"
                };`,
            )
            .join("\n")}\n}\n`;
        }
      } catch {
        // Ignore errors reading component file
      }
    }

    sendJson(palette);
  } catch (e) {
    sendError(e instanceof Error ? e.message : String(e));
  }
}

/** GET /api/arts/:path/analysis */
async function handleArtAnalysis(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): Promise<void> {
  const artPath = decodeURIComponent(match[1]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  try {
    const resolvedComponentPath =
      art.isInline && art.componentPath
        ? art.componentPath
        : art.metadata.component
          ? path.isAbsolute(art.metadata.component)
            ? art.metadata.component
            : path.resolve(path.dirname(artPath), art.metadata.component)
          : null;

    if (resolvedComponentPath) {
      const source = await fs.promises.readFile(resolvedComponentPath, "utf-8");
      const binding = loadNative();
      if (binding.analyzeSfc) {
        const analysis = binding.analyzeSfc(source, {
          filename: resolvedComponentPath,
        });
        sendJson(analysis);
      } else {
        const analysis = analyzeSfcFallback(source, {
          filename: resolvedComponentPath,
        });
        sendJson(analysis);
      }
    } else {
      sendJson({ props: [], emits: [] });
    }
  } catch (e) {
    sendError(e instanceof Error ? e.message : String(e));
  }
}

/** GET /api/arts/:path/docs */
async function handleArtDocs(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): Promise<void> {
  const artPath = decodeURIComponent(match[1]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  try {
    const source = await fs.promises.readFile(artPath, "utf-8");
    const binding = loadNative();
    if (binding.generateArtDoc) {
      const doc = binding.generateArtDoc(source, {
        filename: artPath,
      });
      // Replace Self with component name and format indentation
      let markdown = doc.markdown || "";
      const componentName = art.metadata.title || "Component";
      markdown = markdown
        .replace(/<Self(\s|>|\/)/g, `<${componentName}$1`)
        .replace(/<\/Self>/g, `</${componentName}>`);
      // Fix indentation in code blocks
      markdown = markdown.replace(
        /```(\w*)\n([\s\S]*?)```/g,
        (_match: string, lang: string, code: string) => {
          const lines = code.split("\n");
          let minIndent = Infinity;
          for (const line of lines) {
            if (line.trim()) {
              const indent = line.match(/^(\s*)/)?.[1].length || 0;
              minIndent = Math.min(minIndent, indent);
            }
          }
          if (minIndent === Infinity) minIndent = 0;
          let formatted: string;
          if (minIndent > 0) {
            formatted = lines.map((line: string) => line.slice(minIndent)).join("\n");
          } else {
            let restIndent = Infinity;
            for (let i = 1; i < lines.length; i++) {
              if (lines[i].trim()) {
                const indent = lines[i].match(/^(\s*)/)?.[1].length || 0;
                restIndent = Math.min(restIndent, indent);
              }
            }
            if (restIndent === Infinity || restIndent === 0) {
              formatted = lines.join("\n");
            } else {
              formatted = lines
                .map((line: string, i: number) => (i === 0 ? line : line.slice(restIndent)))
                .join("\n");
            }
          }
          return "```" + lang + "\n" + formatted + "```";
        },
      );
      sendJson({ ...doc, markdown });
    } else {
      sendJson({
        markdown: "",
        title: art.metadata.title,
        variant_count: art.variants.length,
      });
    }
  } catch (e) {
    sendError(e instanceof Error ? e.message : String(e));
  }
}

/** GET /api/arts/:path/variants/:name/a11y */
function handleArtA11y(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): void {
  const artPath = decodeURIComponent(match[1]);
  const _variantName = decodeURIComponent(match[2]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  // Return empty a11y results (populated after VRT --a11y run)
  sendJson({ violations: [], passes: 0, incomplete: 0 });
}
