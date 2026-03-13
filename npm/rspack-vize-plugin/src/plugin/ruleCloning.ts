/**
 * Rule Cloning — VueLoaderPlugin-style automatic rule injection
 *
 * Scans `compiler.options.module.rules` and rewrites them so that users
 * only need to write a single `.vue` rule (with the main vize loader) plus
 * ordinary CSS / preprocessor rules.  The plugin automatically generates
 * the `oneOf` branches needed to route `?vue&type=style` sub-requests to
 * the correct style pipeline.
 *
 * The algorithm mirrors what `VueLoaderPlugin` does in `vue-loader`:
 *
 * 1. Find the rule whose `use` chain contains the vize main loader
 *    (identified by loader path containing `@vizejs/rspack-plugin/loader`
 *    or the literal module string).
 *
 * 2. Find CSS / preprocessor rules (test matches `.css`, `.scss`, etc.)
 *    that are **not** already scoped to `.vue` sub-requests.
 *
 * 3. Clone those CSS rules so the clones match
 *    `resourceQuery: /vue&type=style/` with appropriate `lang=` conditions.
 *
 * 4. Wrap the original `.vue` rule in `oneOf`:
 *      - cloned style rules (from step 3)
 *      - the vize style-loader fallback for plain CSS
 *      - the original main loader as the catch-all
 *
 * 5. Mark the original CSS rules with `resourceQuery: { not: [/vue/] }` so
 *    they don't accidentally match `.vue` style sub-requests.
 */

import type { RuleSetRule, RuleSetUseItem } from "@rspack/core";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const VIZE_LOADER_IDENT = "@vizejs/rspack-plugin/loader";

const VIZE_STYLE_LOADER_IDENT = "@vizejs/rspack-plugin/style-loader";

/** Regex extensions we know how to clone into Vue style sub-request rules. */
const STYLE_EXTENSION_MAP: Record<string, string> = {
  "\\.css$": "css",
  "\\.scss$": "scss",
  "\\.sass$": "sass",
  "\\.less$": "less",
  "\\.styl(us)?$": "styl",
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export interface RuleCloningResult {
  /** Whether rule cloning was performed */
  applied: boolean;
  /** Number of CSS rules cloned */
  clonedCount: number;
  /** Warnings to emit through the infrastructure logger */
  warnings: string[];
}

/**
 * Mutates `rules` in-place: rewrites the .vue rule into a `oneOf` form and
 * clones existing CSS rules for Vue style sub-requests.
 *
 * @param rules  `compiler.options.module.rules` (mutable)
 * @param nativeCss  Whether `experiments.css` is enabled
 */
export function applyRuleCloning(
  rules: (RuleSetRule | "...")[],
  nativeCss: boolean,
): RuleCloningResult {
  const warnings: string[] = [];

  // ── Step 1: locate the .vue rule that uses the vize main loader ──────────
  const vueRuleIndex = rules.findIndex((r) => r !== "..." && isVueMainRule(r));

  if (vueRuleIndex === -1) {
    // No vize loader found — nothing to do.  The user might be using
    // manual oneOf configuration instead.
    return { applied: false, clonedCount: 0, warnings };
  }

  const vueRule = rules[vueRuleIndex] as RuleSetRule;

  // If the rule already has `oneOf`, it's either been processed by a previous
  // plugin apply or the user is using manual configuration.
  if (vueRule.oneOf) {
    return { applied: false, clonedCount: 0, warnings };
  }

  // ── Step 2: find all CSS / preprocessor rules ────────────────────────────
  const cssRuleEntries: Array<{
    index: number;
    rule: RuleSetRule;
    lang: string;
  }> = [];

  for (let i = 0; i < rules.length; i++) {
    if (i === vueRuleIndex) continue;
    const rule = rules[i];
    if (rule === "...") continue;

    const lang = detectStyleLang(rule);
    if (lang) {
      cssRuleEntries.push({ index: i, rule, lang });
    }
  }

  // ── Step 3: clone CSS rules for vue style sub-requests ───────────────────
  const clonedStyleRules: RuleSetRule[] = [];

  for (const entry of cssRuleEntries) {
    const cloned = cloneRuleForVueStyle(entry.rule, entry.lang, nativeCss);
    if (cloned) {
      clonedStyleRules.push(cloned);
    }
  }

  // Always add a fallback rule for plain CSS coming from <style> blocks
  // that don't match any user-provided CSS rule.
  const hasCssFallback = clonedStyleRules.some(
    (r) =>
      r.resourceQuery instanceof RegExp &&
      r.resourceQuery.test("vue&type=style&index=0&lang=css"),
  );

  if (!hasCssFallback) {
    clonedStyleRules.push(createFallbackStyleRule(nativeCss));
  }

  // ── Step 4: build oneOf ──────────────────────────────────────────────────
  // Extract the original use chain from the .vue rule for the main loader slot.
  // Handle both `use` and `loader`/`options` shorthand forms.
  const mainLoaderBranch: RuleSetRule = {
    use: normalizeUseFromRule(vueRule),
  };

  const oneOf: RuleSetRule[] = [...clonedStyleRules, mainLoaderBranch];

  // Replace the original .vue rule with the oneOf version.
  rules[vueRuleIndex] = {
    test: vueRule.test,
    oneOf,
  };

  // ── Step 5: exclude vue sub-requests from original CSS rules ─────────────
  for (const entry of cssRuleEntries) {
    addVueExclusion(entry.rule);
  }

  return {
    applied: true,
    clonedCount: clonedStyleRules.length,
    warnings,
  };
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Check whether a rule's `use` chain contains the vize main loader.
 * Handles both `rule.use` and `rule.loader` shorthand.
 */
function isVueMainRule(rule: RuleSetRule): boolean {
  // Must target .vue files
  if (!testMatchesVue(rule.test)) return false;

  const uses = normalizeUseFromRule(rule);
  return uses.some((u) => {
    const loader =
      typeof u === "string" ? u : (u as { loader?: string }).loader;
    return loader ? isVizeMainLoader(loader) : false;
  });
}

function isVizeMainLoader(loader: string): boolean {
  // Match the package export path or a resolved file path
  return (
    loader === VIZE_LOADER_IDENT ||
    (loader.includes("rspack-vize-plugin") &&
      loader.includes("loader") &&
      !loader.includes("style-loader"))
  );
}

function testMatchesVue(test: RuleSetRule["test"]): boolean {
  if (!test) return false;
  if (test instanceof RegExp) {
    return test.test("App.vue") || test.test("foo.vue");
  }
  if (typeof test === "string") {
    return test.includes(".vue");
  }
  return false;
}

/**
 * Detect if a rule targets a CSS/preprocessor extension.
 * Returns the language name or null.
 */
function detectStyleLang(rule: RuleSetRule): string | null {
  const test = rule.test;
  if (!test || !(test instanceof RegExp)) return null;

  const testStr = test.source;
  for (const [pattern, lang] of Object.entries(STYLE_EXTENSION_MAP)) {
    if (testStr.includes(pattern) || testStr === pattern) {
      return lang;
    }
  }

  // Also check common patterns
  if (test.test("foo.css") && !test.test("foo.vue")) return "css";
  if (test.test("foo.scss") && !test.test("foo.vue")) return "scss";
  if (test.test("foo.sass") && !test.test("foo.vue")) return "sass";
  if (test.test("foo.less") && !test.test("foo.vue")) return "less";
  if (test.test("foo.styl") && !test.test("foo.vue")) return "styl";

  return null;
}

/**
 * Clone a CSS rule so it matches `?vue&type=style` sub-requests instead.
 *
 * The clone:
 * - Drops `test` (since the parent .vue rule already checked the extension)
 * - Adds `resourceQuery` matching `vue&type=style` + `lang=<lang>`
 * - Prepends the vize style-loader to the `use` chain (rightmost = first executed)
 * - Preserves the `type` field for native CSS mode
 */
function cloneRuleForVueStyle(
  rule: RuleSetRule,
  lang: string,
  nativeCss: boolean,
): RuleSetRule | null {
  const uses = normalizeUseFromRule(rule);
  if (uses.length === 0) return null;

  // Build resourceQuery regex with lookaheads for order-independence
  const resourceQuery = new RegExp(`(?=.*type=style)(?=.*lang=${lang})`);

  // For native CSS mode, we need to set the appropriate type
  // The vize style-loader extracts CSS from the SFC; after that,
  // the user's preprocessor loaders run, and rspack handles the rest.
  const clonedUse: RuleSetUseItem[] = [
    ...deepCloneUse(uses),
    { loader: VIZE_STYLE_LOADER_IDENT },
  ];

  const cloned: RuleSetRule = {
    resourceQuery,
    use: clonedUse,
  };

  // Preserve the `type` if specified, otherwise infer from nativeCss mode
  if (rule.type) {
    cloned.type = rule.type;
  } else if (nativeCss) {
    cloned.type = "css/auto";
  }

  return cloned;
}

/**
 * Also generate a CSS Module variant for rules that could match `<style module>`.
 * This is handled implicitly: native CSS uses `css/auto` which auto-detects modules
 * via the query, and non-native CSS relies on css-loader's `modules.auto` setting.
 */

/**
 * Create a fallback rule for plain `<style>` blocks (lang=css).
 */
function createFallbackStyleRule(nativeCss: boolean): RuleSetRule {
  const resourceQuery = /(?=.*type=style)(?=.*lang=css)/;

  if (nativeCss) {
    return {
      resourceQuery,
      type: "css/auto",
      use: [{ loader: VIZE_STYLE_LOADER_IDENT }],
    };
  }

  // Non-native: just extract the CSS, upstream loaders will handle it
  return {
    resourceQuery,
    type: "javascript/auto",
    use: [{ loader: VIZE_STYLE_LOADER_IDENT }],
  };
}

/**
 * Add `resourceQuery: { not: [/vue/] }` to a rule so it doesn't match
 * Vue style sub-requests.  Mutates the rule in-place.
 */
function addVueExclusion(rule: RuleSetRule): void {
  const existing = rule.resourceQuery;
  if (existing) {
    // If it already has a `not` exclusion for vue, skip
    if (
      typeof existing === "object" &&
      !Array.isArray(existing) &&
      !(existing instanceof RegExp) &&
      "not" in existing
    ) {
      return;
    }
    // Don't overwrite complex existing resourceQuery conditions
    return;
  }
  rule.resourceQuery = { not: [/vue/] };
}

/**
 * Normalize `rule.use` to an array of use items.
 */
function normalizeUse(use: RuleSetRule["use"]): RuleSetUseItem[] {
  if (!use) return [];
  if (Array.isArray(use)) return use as RuleSetUseItem[];
  return [use as RuleSetUseItem];
}

/**
 * Normalize a rule's loaders from either `rule.use` or `rule.loader` (+`rule.options`)
 * into a flat array of use items.  Rspack/webpack accept both forms:
 *   { use: [{ loader, options }] }     — canonical
 *   { loader: "...", options: {...} }  — shorthand
 */
function normalizeUseFromRule(rule: RuleSetRule): RuleSetUseItem[] {
  // Prefer `use` if present
  const fromUse = normalizeUse(rule.use);
  if (fromUse.length > 0) return fromUse;

  // Fall back to `loader` shorthand
  const loader = (rule as Record<string, unknown>).loader as string | undefined;
  if (loader) {
    const options = (rule as Record<string, unknown>).options as
      | Record<string, unknown>
      | undefined;
    return options ? [{ loader, options }] : [loader];
  }

  return [];
}

/**
 * Deep-clone an array of use items to avoid mutating the original rules.
 */
function deepCloneUse(uses: RuleSetUseItem[]): RuleSetUseItem[] {
  return uses.map((u) => {
    if (typeof u === "string") return u;
    if (typeof u === "object" && u !== null) {
      const cloned: Record<string, unknown> = { ...u };
      if ("options" in u && u.options && typeof u.options === "object") {
        cloned.options = { ...(u.options as Record<string, unknown>) };
      }
      return cloned as RuleSetUseItem;
    }
    return u;
  });
}
