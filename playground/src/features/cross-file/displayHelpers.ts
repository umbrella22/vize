import {
  mdiLanguageTypescript,
  mdiVuejs,
  mdiFile,
  mdiClose,
  mdiAlert,
  mdiInformation,
} from "@mdi/js";

export function getFileIcon(filename: string): string {
  if (filename.endsWith(".vue")) return mdiVuejs;
  if (filename.endsWith(".ts")) return mdiLanguageTypescript;
  return mdiFile;
}

export function getSeverityIcon(severity: string): string {
  return severity === "error" ? mdiClose : severity === "warning" ? mdiAlert : mdiInformation;
}

export function getTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    "provide-inject": "Provide/Inject",
    "component-emit": "Component Emit",
    "fallthrough-attrs": "Fallthrough Attrs",
    reactivity: "Reactivity",
    "unique-id": "Unique ID",
    "ssr-boundary": "SSR Boundary",
  };
  return labels[type] || type;
}

export function getTypeColor(type: string): string {
  const colors: Record<string, string> = {
    "provide-inject": "#8b5cf6",
    "component-emit": "#f59e0b",
    "fallthrough-attrs": "#06b6d4",
    reactivity: "#ef4444",
    "unique-id": "#10b981",
    "ssr-boundary": "#3b82f6",
  };
  return colors[type] || "#6b7280";
}
