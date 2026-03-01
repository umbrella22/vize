/**
 * VRT runner using Playwright for browser automation.
 *
 * Manages browser lifecycle, screenshot capture, and baseline comparison
 * for visual regression testing of Musea art file variants.
 */

import type { Browser, BrowserContext, Page } from "playwright";
import type {
  ArtFileInfo,
  VrtOptions,
  ViewportConfig,
  CaptureConfig,
  ComparisonConfig,
  CiConfig,
} from "../types.js";
import fs from "node:fs";
import path from "node:path";
import { PNG } from "pngjs";

import {
  readPng,
  writePng,
  colorDelta,
  isAntiAliased,
  fileExists,
  matchGlob,
} from "./comparison.js";

/**
 * VRT test result for a single variant.
 */
export interface VrtResult {
  artPath: string;
  variantName: string;
  viewport: ViewportConfig;
  passed: boolean;
  snapshotPath: string;
  currentPath?: string;
  diffPath?: string;
  diffPercentage?: number;
  diffPixels?: number;
  totalPixels?: number;
  error?: string;
  isNew?: boolean;
}

/**
 * VRT summary for reporting.
 */
export interface VrtSummary {
  total: number;
  passed: number;
  failed: number;
  new: number;
  skipped: number;
  duration: number;
}

/**
 * Extended VRT options aligned with Rust VrtConfig.
 */
export interface ExtendedVrtOptions extends VrtOptions {
  capture?: CaptureConfig;
  comparison?: ComparisonConfig;
  ci?: CiConfig;
  /** Enable a11y auditing during VRT */
  a11y?: boolean;
}

/**
 * Pixel comparison options.
 */
export interface PixelCompareOptions {
  /** Threshold for color difference (0-1). Default: 0.1 */
  threshold?: number;
  /** Include anti-aliasing in diff. Default: false */
  includeAA?: boolean;
  /** Alpha channel comparison. Default: true */
  alpha?: boolean;
  /** Diff highlight color */
  diffColor?: { r: number; g: number; b: number };
}

/**
 * VRT runner using Playwright.
 */
export class MuseaVrtRunner {
  private options: Required<VrtOptions>;
  private capture: Required<CaptureConfig>;
  private comparison: ComparisonConfig;
  private ci: CiConfig;
  private browser: Browser | null = null;
  private startTime: number = 0;

  constructor(options: ExtendedVrtOptions = {}) {
    this.options = {
      snapshotDir: options.snapshotDir ?? ".vize/snapshots",
      threshold: options.threshold ?? 0.1,
      viewports: options.viewports ?? [
        { width: 1280, height: 720, name: "desktop" },
        { width: 375, height: 667, name: "mobile" },
      ],
    };
    this.capture = {
      fullPage: options.capture?.fullPage ?? false,
      waitForNetwork: options.capture?.waitForNetwork ?? true,
      settleTime: options.capture?.settleTime ?? 100,
      waitSelector: options.capture?.waitSelector ?? ".musea-variant",
      hideElements: options.capture?.hideElements ?? [],
      maskElements: options.capture?.maskElements ?? [],
    };
    this.comparison = options.comparison ?? {};
    this.ci = options.ci ?? {};
  }

  /**
   * Initialize Playwright browser.
   */
  async init(): Promise<void> {
    const { chromium } = await import("playwright");
    this.browser = await chromium.launch({ headless: true });
    this.startTime = Date.now();
  }

  /**
   * Close browser and cleanup.
   */
  async close(): Promise<void> {
    if (this.browser) {
      await this.browser.close();
      this.browser = null;
    }
  }

  /**
   * Alias for init() - used by the plugin API.
   */
  async start(): Promise<void> {
    return this.init();
  }

  /**
   * Alias for close() - used by the plugin API.
   */
  async stop(): Promise<void> {
    return this.close();
  }

  /**
   * Run VRT tests for all Art files.
   */
  async runAllTests(artFiles: ArtFileInfo[], baseUrl: string): Promise<VrtResult[]> {
    if (!this.browser) {
      throw new Error("VRT runner not initialized. Call init() first.");
    }

    const results: VrtResult[] = [];
    const retries = this.ci.retries ?? 0;

    for (const art of artFiles) {
      for (const variant of art.variants) {
        if (variant.skipVrt) {
          continue;
        }

        // Determine viewports: use per-variant viewport if defined, else global viewports
        const viewports = variant.args?.viewport
          ? [variant.args.viewport as ViewportConfig]
          : this.options.viewports;

        for (const viewport of viewports) {
          let result: VrtResult | null = null;
          let attempts = 0;

          while (attempts <= retries) {
            result = await this.captureAndCompare(art, variant.name, viewport, baseUrl);
            if (result.passed || result.isNew || !result.error) {
              break;
            }
            attempts++;
            if (attempts <= retries) {
              console.log(
                `[vrt] Retry ${attempts}/${retries}: ${path.basename(art.path)}/${variant.name}`,
              );
            }
          }

          if (result) {
            results.push(result);
          }
        }
      }
    }

    return results;
  }

  /**
   * Run VRT tests - alias used by the plugin API that accepts options.
   */
  async runTests(
    artFiles: ArtFileInfo[],
    baseUrl: string,
    _options?: { updateSnapshots?: boolean },
  ): Promise<VrtResult[]> {
    const results = await this.runAllTests(artFiles, baseUrl);
    if (_options?.updateSnapshots) {
      await this.updateBaselines(results);
    }
    return results;
  }

  /**
   * Capture screenshot and compare with baseline.
   */
  async captureAndCompare(
    art: ArtFileInfo,
    variantName: string,
    viewport: ViewportConfig,
    baseUrl: string,
  ): Promise<VrtResult> {
    if (!this.browser) {
      throw new Error("VRT runner not initialized. Call init() first.");
    }

    const snapshotDir = this.options.snapshotDir;
    const artBaseName = path.basename(art.path, ".art.vue");
    const viewportName = viewport.name || `${viewport.width}x${viewport.height}`;
    const snapshotName = `${artBaseName}--${variantName}--${viewportName}.png`;
    const snapshotPath = path.join(snapshotDir, snapshotName);
    const currentPath = path.join(snapshotDir, "current", snapshotName);
    const diffPath = path.join(snapshotDir, "diff", snapshotName);

    // Ensure directories exist
    await fs.promises.mkdir(path.dirname(snapshotPath), { recursive: true });
    await fs.promises.mkdir(path.join(snapshotDir, "current"), { recursive: true });
    await fs.promises.mkdir(path.join(snapshotDir, "diff"), { recursive: true });

    let context: BrowserContext | null = null;
    let page: Page | null = null;

    try {
      context = await this.browser.newContext({
        viewport: {
          width: viewport.width,
          height: viewport.height,
        },
        deviceScaleFactor: viewport.deviceScaleFactor ?? 1,
      });
      page = await context.newPage();

      // Navigate to variant preview URL
      const variantUrl = this.buildVariantUrl(baseUrl, art.path, variantName);
      const waitUntil = this.capture.waitForNetwork ? ("networkidle" as const) : ("load" as const);
      await page.goto(variantUrl, { waitUntil });

      // Wait for content to render
      await page.waitForSelector(this.capture.waitSelector, { timeout: 10000 });

      // Additional wait for animations to settle
      await page.waitForTimeout(this.capture.settleTime);

      // Hide elements before capture
      if (this.capture.hideElements.length > 0) {
        for (const selector of this.capture.hideElements) {
          await page.evaluate((sel) => {
            document.querySelectorAll(sel).forEach((el) => {
              (el as HTMLElement).style.visibility = "hidden";
            });
          }, selector);
        }
      }

      // Mask elements before capture (replace with colored box)
      if (this.capture.maskElements.length > 0) {
        for (const selector of this.capture.maskElements) {
          await page.evaluate((sel) => {
            document.querySelectorAll(sel).forEach((el) => {
              const htmlEl = el as HTMLElement;
              htmlEl.style.background = "#ff00ff";
              htmlEl.style.color = "transparent";
              htmlEl.innerHTML = "";
            });
          }, selector);
        }
      }

      // Take screenshot
      await page.screenshot({
        path: currentPath,
        fullPage: this.capture.fullPage,
      });

      // Check if baseline exists
      const hasBaseline = await fileExists(snapshotPath);

      if (!hasBaseline) {
        // First run - save as baseline
        await fs.promises.copyFile(currentPath, snapshotPath);
        return {
          artPath: art.path,
          variantName,
          viewport,
          passed: true,
          snapshotPath,
          currentPath,
          isNew: true,
        };
      }

      // Compare images using pixel comparison
      const comparison = await this.compareImages(snapshotPath, currentPath, diffPath);

      const passed = comparison.diffPercentage <= this.options.threshold;

      return {
        artPath: art.path,
        variantName,
        viewport,
        passed,
        snapshotPath,
        currentPath,
        diffPath: passed ? undefined : diffPath,
        diffPercentage: comparison.diffPercentage,
        diffPixels: comparison.diffPixels,
        totalPixels: comparison.totalPixels,
      };
    } catch (error) {
      return {
        artPath: art.path,
        variantName,
        viewport,
        passed: false,
        snapshotPath,
        error: error instanceof Error ? error.message : String(error),
      };
    } finally {
      if (page) await page.close();
      if (context) await context.close();
    }
  }

  /**
   * Get the Playwright Page for external use (e.g., a11y auditing).
   */
  async createPage(viewport: ViewportConfig): Promise<{ page: Page; context: BrowserContext }> {
    if (!this.browser) {
      throw new Error("VRT runner not initialized. Call init() first.");
    }
    const context = await this.browser.newContext({
      viewport: { width: viewport.width, height: viewport.height },
      deviceScaleFactor: viewport.deviceScaleFactor ?? 1,
    });
    const page = await context.newPage();
    return { page, context };
  }

  /**
   * Update baseline snapshots with current screenshots.
   */
  async updateBaselines(results: VrtResult[]): Promise<number> {
    let updated = 0;
    const snapshotDir = this.options.snapshotDir;
    const currentDir = path.join(snapshotDir, "current");

    for (const result of results) {
      const currentPath = path.join(currentDir, path.basename(result.snapshotPath));

      if (await fileExists(currentPath)) {
        await fs.promises.copyFile(currentPath, result.snapshotPath);
        updated++;
        console.log(`[vrt] Updated: ${path.basename(result.snapshotPath)}`);
      }
    }

    return updated;
  }

  /**
   * Approve specific failed results (update their baselines).
   */
  async approveResults(results: VrtResult[], pattern?: string): Promise<number> {
    const toApprove = pattern
      ? results.filter((r) => {
          const name = `${path.basename(r.artPath, ".art.vue")}/${r.variantName}`;
          return name.includes(pattern) || matchGlob(name, pattern);
        })
      : results.filter((r) => !r.passed && !r.error);

    return this.updateBaselines(toApprove);
  }

  /**
   * Clean orphaned snapshots (no corresponding art/variant).
   */
  async cleanOrphans(artFiles: ArtFileInfo[]): Promise<number> {
    const snapshotDir = this.options.snapshotDir;
    let cleaned = 0;

    try {
      const files = await fs.promises.readdir(snapshotDir);
      const validNames = new Set<string>();

      for (const art of artFiles) {
        const artBaseName = path.basename(art.path, ".art.vue");
        for (const variant of art.variants) {
          if (variant.skipVrt) continue;
          for (const viewport of this.options.viewports) {
            const viewportName = viewport.name || `${viewport.width}x${viewport.height}`;
            validNames.add(`${artBaseName}--${variant.name}--${viewportName}.png`);
          }
        }
      }

      for (const file of files) {
        if (file.endsWith(".png") && !validNames.has(file)) {
          await fs.promises.unlink(path.join(snapshotDir, file));
          cleaned++;
          console.log(`[vrt] Cleaned: ${file}`);
        }
      }
    } catch {
      // Directory may not exist yet
    }

    return cleaned;
  }

  /**
   * Get VRT summary statistics.
   */
  getSummary(results: VrtResult[]): VrtSummary {
    return {
      total: results.length,
      passed: results.filter((r) => r.passed && !r.isNew).length,
      failed: results.filter((r) => !r.passed && !r.error).length,
      new: results.filter((r) => r.isNew).length,
      skipped: results.filter((r) => r.error).length,
      duration: Date.now() - this.startTime,
    };
  }

  /**
   * Build URL for variant preview.
   */
  private buildVariantUrl(baseUrl: string, artPath: string, variantName: string): string {
    const encodedPath = encodeURIComponent(artPath);
    const encodedVariant = encodeURIComponent(variantName);
    return `${baseUrl}/__musea__/preview?art=${encodedPath}&variant=${encodedVariant}`;
  }

  /**
   * Compare two PNG images and generate a diff image.
   * Returns pixel difference statistics.
   */
  private async compareImages(
    baselinePath: string,
    currentPath: string,
    diffPath: string,
  ): Promise<{ diffPixels: number; totalPixels: number; diffPercentage: number }> {
    const baseline = await readPng(baselinePath);
    const current = await readPng(currentPath);

    // Handle size mismatch
    if (baseline.width !== current.width || baseline.height !== current.height) {
      const width = Math.max(baseline.width, current.width);
      const height = Math.max(baseline.height, current.height);
      const diff = new PNG({ width, height });

      // Fill with red to indicate size mismatch
      for (let i = 0; i < diff.data.length; i += 4) {
        diff.data[i] = 255; // R
        diff.data[i + 1] = 0; // G
        diff.data[i + 2] = 0; // B
        diff.data[i + 3] = 255; // A
      }

      await writePng(diff, diffPath);

      return {
        diffPixels: width * height,
        totalPixels: width * height,
        diffPercentage: 100,
      };
    }

    const width = baseline.width;
    const height = baseline.height;
    const totalPixels = width * height;
    const diff = new PNG({ width, height });

    const useAntiAliasing = this.comparison.antiAliasing ?? true;
    const useAlpha = this.comparison.alpha ?? true;
    const diffColor = this.comparison.diffColor ?? { r: 255, g: 0, b: 0 };

    // Pixel comparison
    let diffPixels = 0;
    const threshold = 0.1; // Color difference threshold

    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const idx = (y * width + x) * 4;

        const r1 = baseline.data[idx];
        const g1 = baseline.data[idx + 1];
        const b1 = baseline.data[idx + 2];
        const a1 = useAlpha ? baseline.data[idx + 3] : 255;

        const r2 = current.data[idx];
        const g2 = current.data[idx + 1];
        const b2 = current.data[idx + 2];
        const a2 = useAlpha ? current.data[idx + 3] : 255;

        // Calculate color difference using YIQ color space
        const delta = colorDelta(r1, g1, b1, a1, r2, g2, b2, a2);

        if (delta > threshold * 255 * 255) {
          // Anti-aliasing detection: check if pixel is likely AA
          if (useAntiAliasing && isAntiAliased(baseline, current, x, y, width, height)) {
            // Mark as AA (yellow)
            diff.data[idx] = 255;
            diff.data[idx + 1] = 200;
            diff.data[idx + 2] = 0;
            diff.data[idx + 3] = 128;
          } else {
            // Mark as different
            diffPixels++;
            diff.data[idx] = diffColor.r;
            diff.data[idx + 1] = diffColor.g;
            diff.data[idx + 2] = diffColor.b;
            diff.data[idx + 3] = 255;
          }
        } else {
          // Grayscale for matching pixels
          const gray = Math.round((r2 + g2 + b2) / 3);
          diff.data[idx] = gray;
          diff.data[idx + 1] = gray;
          diff.data[idx + 2] = gray;
          diff.data[idx + 3] = 128; // Semi-transparent
        }
      }
    }

    // Only write diff if there are differences
    if (diffPixels > 0) {
      await writePng(diff, diffPath);
    }

    const diffPercentage = (diffPixels / totalPixels) * 100;

    return {
      diffPixels,
      totalPixels,
      diffPercentage,
    };
  }
}
