import { ref, shallowRef, watch } from "vue";
import { useMessageListener, sendMessage } from "./usePostMessage";

export interface A11yNode {
  html: string;
  target: string[];
  failureSummary?: string;
}

export interface A11yViolation {
  id: string;
  impact: "critical" | "serious" | "moderate" | "minor";
  description: string;
  helpUrl: string;
  nodes: A11yNode[];
}

export interface A11yResult {
  violations: A11yViolation[];
  passes: number;
  incomplete: number;
  error?: string;
}

// Singleton state
const results = shallowRef<Map<string, A11yResult>>(new Map());
const runningKeys = ref<Set<string>>(new Set());
const pendingIframes = new Map<HTMLIFrameElement, string>();

export function useA11y() {
  function init() {
    useMessageListener("musea:a11y-result", (payload: unknown, event: MessageEvent) => {
      const result = payload as A11yResult;
      // Find key by matching event.source to pending iframes
      let matchedKey = "";
      for (const [iframe, key] of pendingIframes) {
        if (iframe.contentWindow === event.source) {
          matchedKey = key;
          pendingIframes.delete(iframe);
          break;
        }
      }
      if (matchedKey) {
        const newMap = new Map(results.value);
        newMap.set(matchedKey, result);
        results.value = newMap;
        const newSet = new Set(runningKeys.value);
        newSet.delete(matchedKey);
        runningKeys.value = newSet;
      }
    });
  }

  function runA11y(iframe: HTMLIFrameElement, key: string) {
    if (runningKeys.value.has(key)) return;
    const newSet = new Set(runningKeys.value);
    newSet.add(key);
    runningKeys.value = newSet;
    pendingIframes.set(iframe, key);
    sendMessage(iframe, "musea:run-a11y", {});
  }

  function isKeyRunning(key: string): boolean {
    return runningKeys.value.has(key);
  }

  function getResult(key: string): A11yResult | undefined {
    return results.value.get(key);
  }

  function runA11yAsync(iframe: HTMLIFrameElement, key: string): Promise<A11yResult> {
    return new Promise<A11yResult>((resolve, reject) => {
      const timeout = setTimeout(() => {
        stop();
        const newSet = new Set(runningKeys.value);
        newSet.delete(key);
        runningKeys.value = newSet;
        reject(new Error("A11y test timed out after 30s"));
      }, 30000);

      const stop = watch(results, (newResults) => {
        const result = newResults.get(key);
        if (result) {
          clearTimeout(timeout);
          stop();
          resolve(result);
        }
      });

      // Trigger the test (reuses existing runA11y logic)
      runA11y(iframe, key);
    });
  }

  function clearResults() {
    results.value = new Map();
  }

  return {
    results,
    runningKeys,
    init,
    runA11y,
    runA11yAsync,
    isKeyRunning,
    getResult,
    clearResults,
  };
}
