import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { createVizeVueRules } from "./rules.js";

describe("createVizeVueRules", () => {
  test("generates rules with default options", () => {
    const rules = createVizeVueRules();
    assert.ok(Array.isArray(rules));
    assert.ok(rules.length >= 1);
    // First rule should be the .vue oneOf rule
    const vueRule = rules[0] as { test: RegExp; oneOf: unknown[] };
    assert.ok(vueRule.test instanceof RegExp);
    assert.ok(vueRule.test.test("App.vue"));
    assert.ok(Array.isArray(vueRule.oneOf));
  });

  test("preprocessorOptions passes options to sass-loader (nativeCss)", () => {
    const rules = createVizeVueRules({
      nativeCss: true,
      styleLanguages: ["scss"],
      preprocessorOptions: {
        scss: {
          additionalData: `@use "src/styles" as *;`,
          sassOptions: { includePaths: ["./src"], quietDeps: true },
        },
      },
    });

    const vueRule = rules[0] as { oneOf: Record<string, unknown>[] };
    // Find a rule that has scss in its resourceQuery and has use array
    const scssRule = vueRule.oneOf.find((rule) => {
      const rq = rule.resourceQuery as RegExp;
      return rq && rq.test("type=style&lang=scss");
    });

    assert.ok(scssRule, "should have a rule matching lang=scss");
    const use = scssRule!.use as Array<Record<string, unknown> | string>;
    // The first entry should be the resolved preprocessor loader with options
    const preprocessorEntry = use[0];
    assert.equal(typeof preprocessorEntry, "object");
    assert.equal((preprocessorEntry as Record<string, unknown>).loader, "sass-loader");
    const opts = (preprocessorEntry as Record<string, unknown>).options as Record<string, unknown>;
    assert.equal(opts.additionalData, `@use "src/styles" as *;`);
    assert.deepEqual(opts.sassOptions, { includePaths: ["./src"], quietDeps: true });
  });

  test("preprocessorOptions passes options to sass-loader (non-nativeCss)", () => {
    const rules = createVizeVueRules({
      nativeCss: false,
      styleLanguages: ["scss"],
      preprocessorOptions: {
        scss: {
          additionalData: `$color: red;`,
        },
      },
    });

    const vueRule = rules[0] as { oneOf: Record<string, unknown>[] };
    const scssRule = vueRule.oneOf.find((rule) => {
      const rq = rule.resourceQuery as RegExp;
      return rq && rq.test("type=style&lang=scss") && !rq.test("module");
    });

    assert.ok(scssRule, "should have a non-module rule matching lang=scss");
    const use = scssRule!.use as Array<Record<string, unknown> | string>;
    // In non-native mode: [inject/extract, css-loader, preprocessor, vizeStyleLoader]
    // The preprocessor is at index 2
    const preprocessorEntry = use[2];
    assert.equal(typeof preprocessorEntry, "object");
    assert.equal((preprocessorEntry as Record<string, unknown>).loader, "sass-loader");
    const opts = (preprocessorEntry as Record<string, unknown>).options as Record<string, unknown>;
    assert.equal(opts.additionalData, "$color: red;");
  });

  test("preprocessorOptions not provided keeps bare string loader", () => {
    const rules = createVizeVueRules({
      nativeCss: true,
      styleLanguages: ["scss"],
    });

    const vueRule = rules[0] as { oneOf: Record<string, unknown>[] };
    const scssRule = vueRule.oneOf.find((rule) => {
      const rq = rule.resourceQuery as RegExp;
      return rq && rq.test("type=style&lang=scss");
    });

    assert.ok(scssRule);
    const use = scssRule!.use as Array<Record<string, unknown> | string>;
    // Without preprocessorOptions, the preprocessor loader should remain a bare string
    assert.equal(use[0], "sass-loader");
  });

  test("preprocessorOptions for less-loader", () => {
    const rules = createVizeVueRules({
      nativeCss: true,
      styleLanguages: ["less"],
      preprocessorOptions: {
        less: {
          math: "always",
          globalVars: { primaryColor: "#1890ff" },
        },
      },
    });

    const vueRule = rules[0] as { oneOf: Record<string, unknown>[] };
    const lessRule = vueRule.oneOf.find((rule) => {
      const rq = rule.resourceQuery as RegExp;
      return rq && rq.test("type=style&lang=less");
    });

    assert.ok(lessRule);
    const use = lessRule!.use as Array<Record<string, unknown> | string>;
    const entry = use[0] as Record<string, unknown>;
    assert.equal(entry.loader, "less-loader");
    assert.deepEqual(entry.options, {
      math: "always",
      globalVars: { primaryColor: "#1890ff" },
    });
  });

  test("typescript: true adds SWC post-processing rule", () => {
    const rules = createVizeVueRules({ typescript: true });
    assert.ok(rules.length >= 2);
    const tsRule = rules[1] as Record<string, unknown>;
    assert.equal(tsRule.enforce, "post");
    assert.equal(tsRule.loader, "builtin:swc-loader");
  });

  test("typescript: false does not add post-processing rule", () => {
    const rules = createVizeVueRules({ typescript: false });
    assert.equal(rules.length, 1);
  });

  test("last oneOf entry is the main vize loader fallback", () => {
    const rules = createVizeVueRules();
    const vueRule = rules[0] as { oneOf: Record<string, unknown>[] };
    const lastOneOf = vueRule.oneOf[vueRule.oneOf.length - 1];
    const use = lastOneOf.use as Array<Record<string, unknown>>;
    assert.equal(use[0].loader, "@vizejs/rspack-plugin/loader");
  });
});
