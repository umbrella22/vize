<script setup lang="ts">
import { ref, watch } from "vue";
import { fetchA11y } from "../api";

const props = defineProps<{
  artPath: string;
  variantName?: string;
}>();

const count = ref<number | null>(null);
const severity = ref<string>("none");

watch(
  () => [props.artPath, props.variantName],
  async ([path, variant]) => {
    if (!path || !variant) {
      count.value = null;
      return;
    }
    try {
      const data = await fetchA11y(path as string, variant as string);
      count.value = data.violations.length;
      if (data.violations.length > 0) {
        const hasCritical = data.violations.some((v) => v.impact === "critical");
        const hasSerious = data.violations.some((v) => v.impact === "serious");
        severity.value = hasCritical ? "critical" : hasSerious ? "serious" : "moderate";
      } else {
        severity.value = "none";
      }
    } catch {
      count.value = null;
    }
  },
  { immediate: true },
);
</script>

<template>
  <span
    v-if="count !== null && count > 0"
    class="a11y-badge"
    :class="'severity-' + severity"
    :title="`${count} accessibility violation${count !== 1 ? 's' : ''}`"
  >
    {{ count }}
  </span>
</template>

<style scoped>
.a11y-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 0.25rem;
  border-radius: 9px;
  font-size: 0.625rem;
  font-weight: 700;
  line-height: 1;
}

.severity-moderate {
  background: rgba(251, 191, 36, 0.2);
  color: var(--musea-warning);
}

.severity-serious {
  background: rgba(248, 113, 113, 0.2);
  color: var(--musea-error);
}

.severity-critical {
  background: rgba(248, 113, 113, 0.3);
  color: var(--musea-error);
}
</style>
