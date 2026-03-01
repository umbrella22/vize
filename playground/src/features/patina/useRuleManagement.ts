import { ref, computed, type Ref } from "vue";
import type { LintRule } from "../../wasm/index";

const STORAGE_KEY = "vize-patina-rules-config";

export function useRuleManagement(rules: Ref<LintRule[]>, lint: () => void) {
  const enabledRules = ref<Set<string>>(new Set());
  const severityOverrides = ref<Map<string, "error" | "warning" | "off">>(new Map());

  // Rule filtering state
  const selectedCategory = ref<string>("all");
  const searchQuery = ref("");

  const categories = computed(() => {
    const cats = new Set(rules.value.map((r) => r.category));
    return ["all", ...Array.from(cats).sort()];
  });

  const filteredRules = computed(() => {
    return rules.value.filter((rule) => {
      const matchesCategory =
        selectedCategory.value === "all" || rule.category === selectedCategory.value;
      const matchesSearch =
        searchQuery.value === "" ||
        rule.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
        rule.description.toLowerCase().includes(searchQuery.value.toLowerCase());
      return matchesCategory && matchesSearch;
    });
  });

  /** Load saved rule configuration from localStorage */
  function loadRuleConfig() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const config = JSON.parse(saved);
        enabledRules.value = new Set(config.enabledRules || []);
        severityOverrides.value = new Map(Object.entries(config.severityOverrides || {}));
      }
    } catch (e) {
      console.warn("Failed to load rule config:", e);
    }
  }

  /** Save rule configuration to localStorage */
  function saveRuleConfig() {
    try {
      const config = {
        enabledRules: Array.from(enabledRules.value),
        severityOverrides: Object.fromEntries(severityOverrides.value),
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
    } catch (e) {
      console.warn("Failed to save rule config:", e);
    }
  }

  /** Initialize all rules as enabled when rules are loaded */
  function initializeRuleState() {
    if (enabledRules.value.size === 0 && rules.value.length > 0) {
      rules.value.forEach((rule) => {
        enabledRules.value.add(rule.name);
      });
      saveRuleConfig();
    }
  }

  /** Toggle rule enabled state */
  function toggleRule(ruleName: string) {
    if (enabledRules.value.has(ruleName)) {
      enabledRules.value.delete(ruleName);
    } else {
      enabledRules.value.add(ruleName);
    }
    saveRuleConfig();
    lint();
  }

  /** Toggle all rules in a category */
  function toggleCategory(category: string, enabled: boolean) {
    const categoryRules = rules.value.filter((r) => r.category === category);
    categoryRules.forEach((rule) => {
      if (enabled) {
        enabledRules.value.add(rule.name);
      } else {
        enabledRules.value.delete(rule.name);
      }
    });
    saveRuleConfig();
    lint();
  }

  /** Enable all rules */
  function enableAllRules() {
    rules.value.forEach((rule) => {
      enabledRules.value.add(rule.name);
    });
    saveRuleConfig();
    lint();
  }

  /** Disable all rules */
  function disableAllRules() {
    enabledRules.value.clear();
    saveRuleConfig();
    lint();
  }

  /** Check if all rules in a category are enabled */
  function isCategoryFullyEnabled(category: string): boolean {
    const categoryRules = rules.value.filter((r) => r.category === category);
    return categoryRules.every((rule) => enabledRules.value.has(rule.name));
  }

  /** Check if some (but not all) rules in a category are enabled */
  function isCategoryPartiallyEnabled(category: string): boolean {
    const categoryRules = rules.value.filter((r) => r.category === category);
    const enabledCount = categoryRules.filter((rule) => enabledRules.value.has(rule.name)).length;
    return enabledCount > 0 && enabledCount < categoryRules.length;
  }

  return {
    enabledRules,
    severityOverrides,
    selectedCategory,
    searchQuery,
    categories,
    filteredRules,
    loadRuleConfig,
    saveRuleConfig,
    initializeRuleState,
    toggleRule,
    toggleCategory,
    enableAllRules,
    disableAllRules,
    isCategoryFullyEnabled,
    isCategoryPartiallyEnabled,
  };
}
