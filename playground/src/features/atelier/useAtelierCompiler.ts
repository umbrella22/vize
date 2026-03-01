import { ref, computed, watch } from "vue";
import { PRESETS, type PresetKey, type InputMode } from "../../presets";
import type {
  CompilerOptions,
  CompileResult,
  SfcCompileResult,
  CssCompileResult,
  CssCompileOptions,
  WasmModule,
} from "../../wasm/index";
import { formatCode, formatCss, transpileToJs } from "./formatters";
import { mapToObject, filterAstProperties } from "./astHelpers";
import { useClipboard } from "../../utils/useClipboard";

type TabType = "code" | "ast" | "bindings" | "tokens" | "helpers" | "sfc" | "css";

export function useAtelierCompiler(getCompiler: () => WasmModule | null) {
  const { copyToClipboard } = useClipboard();

  const inputMode = ref<InputMode>("sfc");
  const source = ref(PRESETS.propsDestructure.code);
  const output = ref<CompileResult | null>(null);
  const sfcResult = ref<SfcCompileResult | null>(null);
  const error = ref<string | null>(null);
  const options = ref<CompilerOptions>({
    mode: "module",
    ssr: false,
    scriptExt: "preserve",
  });
  const activeTab = ref<TabType>("code");
  const isCompiling = ref(false);
  const selectedPreset = ref<PresetKey>("propsDestructure");
  const compileTime = ref<number | null>(null);
  const cssResult = ref<CssCompileResult | null>(null);
  const cssOptions = ref<CssCompileOptions>({
    scoped: false,
    scopeId: "data-v-12345678",
    minify: false,
  });
  const formattedCode = ref<string>("");
  const formattedCss = ref<string>("");
  const formattedJsCode = ref<string>("");
  const codeViewMode = ref<"ts" | "js">("ts");
  const astHideLoc = ref(true);
  const astHideSource = ref(true);
  const astCollapsed = ref(false);

  const editorLanguage = computed(() => (inputMode.value === "sfc" ? "vue" : "html"));

  const astJson = computed(() => {
    if (!output.value) return "{}";
    const ast = mapToObject(output.value.ast);
    const filtered = filterAstProperties(ast, astHideLoc.value, astHideSource.value);
    return JSON.stringify(filtered, null, astCollapsed.value ? 0 : 2);
  });

  const isTypeScript = computed(() => {
    if (!sfcResult.value?.descriptor) return false;
    const scriptSetup = sfcResult.value.descriptor.scriptSetup;
    const script = sfcResult.value.descriptor.script;
    const lang = scriptSetup?.lang || script?.lang;
    return lang === "ts" || lang === "tsx";
  });

  const bindingsSummary = computed(() => {
    const bindings = sfcResult.value?.script?.bindings?.bindings;
    if (!bindings) return {};
    const summary: Record<string, number> = {};
    for (const type of Object.values(bindings)) {
      summary[type as string] = (summary[type as string] || 0) + 1;
    }
    return summary;
  });

  const groupedBindings = computed(() => {
    const bindings = sfcResult.value?.script?.bindings?.bindings;
    if (!bindings) return {};
    const groups: Record<string, string[]> = {};
    for (const [name, type] of Object.entries(bindings)) {
      if (!groups[type as string]) groups[type as string] = [];
      groups[type as string].push(name);
    }
    return groups;
  });

  async function compile() {
    const compiler = getCompiler();
    if (!compiler) return;

    isCompiling.value = true;
    error.value = null;

    try {
      const startTime = performance.now();

      if (inputMode.value === "sfc") {
        try {
          const result = compiler.compileSfc(source.value, options.value);
          compileTime.value = performance.now() - startTime;
          sfcResult.value = result;

          if (result?.descriptor?.styles?.length > 0) {
            const allCss = result.descriptor.styles
              .map((s: { content: string }) => s.content)
              .join("\n");
            const hasScoped = result.descriptor.styles.some((s: { scoped?: boolean }) => s.scoped);
            const css = compiler.compileCss(allCss, {
              ...cssOptions.value,
              scoped: hasScoped || cssOptions.value.scoped,
            });
            cssResult.value = css;
            formattedCss.value = await formatCss(css.code);
          } else {
            cssResult.value = null;
            formattedCss.value = "";
          }

          if (result?.script?.code) {
            output.value = {
              code: result.script.code,
              preamble: result.template?.preamble || "",
              ast: result.template?.ast || {},
              helpers: result.template?.helpers || [],
            };
            const scriptLang =
              result.descriptor.scriptSetup?.lang || result.descriptor.script?.lang;
            const usesTs = scriptLang === "ts" || scriptLang === "tsx";
            formattedCode.value = await formatCode(
              result.script.code,
              usesTs ? "typescript" : "babel",
            );
            if (usesTs) {
              const jsCode = transpileToJs(result.script.code);
              formattedJsCode.value = await formatCode(jsCode, "babel");
            } else {
              formattedJsCode.value = "";
            }
          } else if (result?.template) {
            output.value = result.template;
            formattedCode.value = await formatCode(result.template.code, "babel");
            formattedJsCode.value = "";
          } else {
            output.value = null;
            formattedCode.value = "";
            formattedJsCode.value = "";
          }
        } catch (sfcError) {
          console.error("SFC compile error:", sfcError);
          throw sfcError;
        }
      } else {
        const result = compiler.compile(source.value, options.value);
        compileTime.value = performance.now() - startTime;
        output.value = result;
        sfcResult.value = null;
        formattedCode.value = await formatCode(result.code, "babel");
        formattedCss.value = "";
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      isCompiling.value = false;
    }
  }

  function handlePresetChange(key: string) {
    const preset = PRESETS[key];
    selectedPreset.value = key;
    inputMode.value = preset.mode;
    source.value = preset.code;
    if (preset.mode === "sfc") {
      activeTab.value = "code";
    }
  }

  function copyFullOutput() {
    if (!output.value) return;
    const fullOutput = `
=== COMPILER OUTPUT ===
Compile Time: ${compileTime?.value?.toFixed(4) ?? "N/A"}ms

=== CODE ===
${output.value.code}

=== HELPERS ===
${output.value.helpers?.join("\n") || "None"}`.trim();
    copyToClipboard(fullOutput);
  }

  let compileTimer: ReturnType<typeof setTimeout> | null = null;

  watch(
    [source, options, inputMode],
    () => {
      if (!getCompiler()) return;
      if (compileTimer) clearTimeout(compileTimer);
      compileTimer = setTimeout(compile, 300);
    },
    { deep: true },
  );

  watch(
    cssOptions,
    () => {
      if (sfcResult.value?.descriptor?.styles?.length) {
        void compile();
      }
    },
    { deep: true },
  );

  return {
    inputMode,
    source,
    output,
    sfcResult,
    error,
    options,
    activeTab,
    isCompiling,
    selectedPreset,
    compileTime,
    cssResult,
    cssOptions,
    formattedCode,
    formattedCss,
    formattedJsCode,
    codeViewMode,
    astHideLoc,
    astHideSource,
    astCollapsed,
    editorLanguage,
    astJson,
    isTypeScript,
    bindingsSummary,
    groupedBindings,
    compile,
    handlePresetChange,
    copyFullOutput,
  };
}
