import { describe, test } from "node:test";
import assert from "node:assert/strict";
import "./../test/setup.ts";
import { applyRuleCloning } from "./ruleCloning.js";

describe("applyRuleCloning", () => {
  test("does nothing when no vize loader is found", () => {
    const rules = [{ test: /\.css$/, use: ["style-loader", "css-loader"] }];
    const result = applyRuleCloning(rules as never, true);

    assert.equal(result.applied, false);
    assert.equal(result.clonedCount, 0);
  });

  test("does nothing when .vue rule already has oneOf", () => {
    const rules = [
      {
        test: /\.vue$/,
        oneOf: [
          { resourceQuery: /type=style/, use: ["some-loader"] },
          { use: [{ loader: "@vizejs/rspack-plugin/loader" }] },
        ],
      },
    ];
    const result = applyRuleCloning(rules as never, true);

    assert.equal(result.applied, false);
  });

  test("clones CSS rule and builds oneOf (nativeCss)", () => {
    const rules = [
      { test: /\.css$/, use: ["style-loader", "css-loader"] },
      {
        test: /\.vue$/,
        use: [{ loader: "@vizejs/rspack-plugin/loader" }],
      },
    ];

    const result = applyRuleCloning(rules as never, true);

    assert.equal(result.applied, true);
    assert.ok(result.clonedCount >= 1);

    // The .vue rule should now have oneOf
    const vueRule = rules[1] as Record<string, unknown>;
    assert.ok(Array.isArray(vueRule.oneOf));

    const oneOf = vueRule.oneOf as Array<Record<string, unknown>>;

    // Last entry should be the main loader fallback
    const mainBranch = oneOf[oneOf.length - 1];
    const use = mainBranch.use as Array<Record<string, unknown>>;
    assert.equal(use[0].loader, "@vizejs/rspack-plugin/loader");

    // CSS rule should now have vue exclusion
    const cssRule = rules[0] as Record<string, unknown>;
    assert.ok(cssRule.resourceQuery);
  });

  test("clones SCSS rule alongside CSS (nativeCss)", () => {
    const rules = [
      { test: /\.css$/, use: ["style-loader", "css-loader"] },
      { test: /\.scss$/, use: ["style-loader", "css-loader", "sass-loader"] },
      {
        test: /\.vue$/,
        use: [{ loader: "@vizejs/rspack-plugin/loader" }],
      },
    ];

    const result = applyRuleCloning(rules as never, true);

    assert.equal(result.applied, true);

    const vueRule = rules[2] as Record<string, unknown>;
    const oneOf = vueRule.oneOf as Array<Record<string, unknown>>;

    // Should have cloned CSS + SCSS + possibly fallback + main loader
    assert.ok(oneOf.length >= 3);

    // Find the SCSS cloned rule
    const scssClone = oneOf.find((r) => {
      const rq = r.resourceQuery as RegExp;
      return rq && rq.test("vue&type=style&index=0&lang=scss");
    });
    assert.ok(scssClone, "should have a cloned SCSS rule");

    // The SCSS clone should include vize style-loader at the end of use chain
    const scssUse = scssClone!.use as Array<string | Record<string, unknown>>;
    const lastLoader = scssUse[scssUse.length - 1] as Record<string, unknown>;
    assert.equal(lastLoader.loader, "@vizejs/rspack-plugin/style-loader");

    // Both original rules should have vue exclusion
    assert.ok((rules[0] as Record<string, unknown>).resourceQuery);
    assert.ok((rules[1] as Record<string, unknown>).resourceQuery);
  });

  test("generates css/auto type for native CSS cloned rules", () => {
    const rules = [
      { test: /\.scss$/, use: ["sass-loader"] },
      {
        test: /\.vue$/,
        use: [{ loader: "@vizejs/rspack-plugin/loader" }],
      },
    ];

    const result = applyRuleCloning(rules as never, true);
    assert.equal(result.applied, true);

    const vueRule = rules[1] as Record<string, unknown>;
    const oneOf = vueRule.oneOf as Array<Record<string, unknown>>;

    // Find a cloned style rule
    const styleRule = oneOf.find((r) => r.resourceQuery instanceof RegExp);
    assert.ok(styleRule);
    assert.equal(styleRule!.type, "css/auto");
  });

  test("always adds a CSS fallback rule for plain <style>", () => {
    const rules = [
      { test: /\.scss$/, use: ["sass-loader"] },
      {
        test: /\.vue$/,
        use: [{ loader: "@vizejs/rspack-plugin/loader" }],
      },
    ];

    const result = applyRuleCloning(rules as never, true);
    assert.equal(result.applied, true);

    const vueRule = rules[1] as Record<string, unknown>;
    const oneOf = vueRule.oneOf as Array<Record<string, unknown>>;

    // Should have a fallback that matches lang=css
    const cssFallback = oneOf.find((r) => {
      const rq = r.resourceQuery as RegExp;
      return rq && rq.test("vue&type=style&index=0&lang=css");
    });
    assert.ok(cssFallback, "should have a CSS fallback rule");
  });

  test("preserves original rule type in clone", () => {
    const rules = [
      { test: /\.css$/, type: "css/module", use: ["css-loader"] },
      {
        test: /\.vue$/,
        use: [{ loader: "@vizejs/rspack-plugin/loader" }],
      },
    ];

    const result = applyRuleCloning(rules as never, true);
    assert.equal(result.applied, true);

    const vueRule = rules[1] as Record<string, unknown>;
    const oneOf = vueRule.oneOf as Array<Record<string, unknown>>;
    const cssClone = oneOf.find((r) => {
      const rq = r.resourceQuery as RegExp;
      return rq && rq.test("vue&type=style&index=0&lang=css");
    });
    assert.ok(cssClone);
    assert.equal(cssClone!.type, "css/module");
  });

  test("deep clones use entries to avoid mutating originals", () => {
    const originalOptions = { modules: true };
    const rules = [
      {
        test: /\.css$/,
        use: [{ loader: "css-loader", options: originalOptions }],
      },
      {
        test: /\.vue$/,
        use: [{ loader: "@vizejs/rspack-plugin/loader" }],
      },
    ];

    applyRuleCloning(rules as never, true);

    const vueRule = rules[1] as Record<string, unknown>;
    const oneOf = vueRule.oneOf as Array<Record<string, unknown>>;
    const cssClone = oneOf.find((r) => {
      const rq = r.resourceQuery as RegExp;
      return rq && rq.test("vue&type=style&index=0&lang=css");
    });
    const clonedUse = cssClone!.use as Array<Record<string, unknown>>;
    const clonedLoader = clonedUse[0] as Record<string, unknown>;

    // Mutating the clone should not affect the original
    (clonedLoader.options as Record<string, unknown>).modules = false;
    assert.equal(originalOptions.modules, true);
  });

  test("handles resolved file paths for vize loader", () => {
    const rules = [
      { test: /\.css$/, use: ["css-loader"] },
      {
        test: /\.vue$/,
        use: [
          {
            loader:
              "/path/to/node_modules/@vizejs/rspack-vize-plugin/dist/loader/index.js",
          },
        ],
      },
    ];

    const result = applyRuleCloning(rules as never, true);
    assert.equal(result.applied, true);
  });

  test("skips '...' entries in rules array", () => {
    const rules: unknown[] = [
      "...",
      { test: /\.css$/, use: ["css-loader"] },
      {
        test: /\.vue$/,
        use: [{ loader: "@vizejs/rspack-plugin/loader" }],
      },
    ];

    const result = applyRuleCloning(rules as never, true);
    assert.equal(result.applied, true);
  });

  test("handles `loader` shorthand (no `use` array)", () => {
    const rules = [
      {
        test: /\.scss$/,
        type: "css/auto",
        use: [
          {
            loader: "sass-loader",
            options: { sassOptions: { quietDeps: true } },
          },
        ],
      },
      {
        test: /\.vue$/,
        loader: "@vizejs/rspack-plugin/loader",
      },
    ];

    const result = applyRuleCloning(rules as never, true);

    assert.equal(result.applied, true);
    assert.ok(result.clonedCount >= 1);

    // The .vue rule should now have oneOf
    const vueRule = rules[1] as Record<string, unknown>;
    assert.ok(Array.isArray(vueRule.oneOf));

    const oneOf = vueRule.oneOf as Array<Record<string, unknown>>;

    // Last entry (main loader fallback) should have the loader in its use array
    const mainBranch = oneOf[oneOf.length - 1];
    const use = mainBranch.use as Array<string | Record<string, unknown>>;
    assert.equal(use[0], "@vizejs/rspack-plugin/loader");

    // SCSS clone should exist
    const scssClone = oneOf.find((r) => {
      const rq = r.resourceQuery as RegExp;
      return rq && rq.test("vue&type=style&index=0&lang=scss");
    });
    assert.ok(scssClone, "should have a cloned SCSS rule");
  });

  test("handles `loader` + `options` shorthand", () => {
    const rules = [
      { test: /\.css$/, use: ["css-loader"] },
      {
        test: /\.vue$/,
        loader: "@vizejs/rspack-plugin/loader",
        options: { sourceMap: true },
      },
    ];

    const result = applyRuleCloning(rules as never, true);
    assert.equal(result.applied, true);

    const vueRule = rules[1] as Record<string, unknown>;
    const oneOf = vueRule.oneOf as Array<Record<string, unknown>>;
    const mainBranch = oneOf[oneOf.length - 1];
    const use = mainBranch.use as Array<Record<string, unknown>>;

    // Should be an object entry with loader + options
    assert.equal(use[0].loader, "@vizejs/rspack-plugin/loader");
    assert.deepEqual(use[0].options, { sourceMap: true });
  });
});
