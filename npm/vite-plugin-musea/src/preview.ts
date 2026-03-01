/**
 * Preview module and HTML generation for Musea component previews.
 *
 * Generates the JavaScript modules that mount Vue components in preview iframes,
 * as well as the HTML wrapper pages for those previews.
 */

import type { ArtFileInfo, ArtVariant } from "./types.js";
import { escapeHtml, escapeTemplate } from "./utils.js";

// Addon initialization code injected into preview iframe modules.
// Shared between generatePreviewModule and generatePreviewModuleWithProps.
const MUSEA_ADDONS_INIT_CODE = `
function __museaInitAddons(container, variantName) {
  // === DOM event capture ===
  // Note: mousemove, mouseenter, mouseleave, pointermove are excluded as they are too noisy
  const CAPTURE_EVENTS = ['click','dblclick','input','change','submit','focus','blur','keydown','keyup','mousedown','mouseup','wheel','contextmenu','pointerdown','pointerup'];
  for (const evt of CAPTURE_EVENTS) {
    container.addEventListener(evt, (e) => {
      // Extract raw event properties
      const rawEvent = {
        type: e.type,
        bubbles: e.bubbles,
        cancelable: e.cancelable,
        composed: e.composed,
        defaultPrevented: e.defaultPrevented,
        eventPhase: e.eventPhase,
        isTrusted: e.isTrusted,
        timeStamp: e.timeStamp,
      };
      // Mouse/Pointer event properties
      if ('clientX' in e) {
        rawEvent.clientX = e.clientX;
        rawEvent.clientY = e.clientY;
        rawEvent.screenX = e.screenX;
        rawEvent.screenY = e.screenY;
        rawEvent.pageX = e.pageX;
        rawEvent.pageY = e.pageY;
        rawEvent.offsetX = e.offsetX;
        rawEvent.offsetY = e.offsetY;
        rawEvent.button = e.button;
        rawEvent.buttons = e.buttons;
        rawEvent.altKey = e.altKey;
        rawEvent.ctrlKey = e.ctrlKey;
        rawEvent.metaKey = e.metaKey;
        rawEvent.shiftKey = e.shiftKey;
      }
      // Keyboard event properties
      if ('key' in e) {
        rawEvent.key = e.key;
        rawEvent.code = e.code;
        rawEvent.repeat = e.repeat;
        rawEvent.altKey = e.altKey;
        rawEvent.ctrlKey = e.ctrlKey;
        rawEvent.metaKey = e.metaKey;
        rawEvent.shiftKey = e.shiftKey;
      }
      // Input event properties
      if ('inputType' in e) {
        rawEvent.inputType = e.inputType;
        rawEvent.data = e.data;
      }
      // Wheel event properties
      if ('deltaX' in e) {
        rawEvent.deltaX = e.deltaX;
        rawEvent.deltaY = e.deltaY;
        rawEvent.deltaZ = e.deltaZ;
        rawEvent.deltaMode = e.deltaMode;
      }
      const payload = {
        name: evt,
        target: e.target?.tagName,
        timestamp: Date.now(),
        source: 'dom',
        rawEvent,
        variantName
      };
      if (e.target && 'value' in e.target) {
        payload.value = e.target.value;
      }
      window.parent.postMessage({ type: 'musea:event', payload }, '*');
    }, true);
  }

  // === Message handler for parent commands ===
  let measureActive = false;
  let measureOverlay = null;
  let measureLabel = null;

  function toggleStyleById(id, enabled, css) {
    let el = document.getElementById(id);
    if (enabled) {
      if (!el) {
        el = document.createElement('style');
        el.id = id;
        el.textContent = css;
        document.head.appendChild(el);
      }
    } else {
      if (el) el.remove();
    }
  }

  function createMeasureOverlay() {
    if (measureOverlay) return;
    measureOverlay = document.createElement('div');
    measureOverlay.id = 'musea-measure-overlay';
    measureOverlay.style.cssText = 'position:fixed;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:99999;';
    document.body.appendChild(measureOverlay);

    measureLabel = document.createElement('div');
    measureLabel.className = 'musea-measure-label';
    measureLabel.style.cssText = 'position:fixed;background:#333;color:#fff;font-size:11px;padding:2px 6px;border-radius:3px;pointer-events:none;z-index:100000;display:none;';
    document.body.appendChild(measureLabel);
  }

  function removeMeasureOverlay() {
    if (measureOverlay) { measureOverlay.remove(); measureOverlay = null; }
    if (measureLabel) { measureLabel.remove(); measureLabel = null; }
  }

  function onMeasureMouseMove(e) {
    if (!measureActive || !measureOverlay) return;
    const el = document.elementFromPoint(e.clientX, e.clientY);
    if (!el || el === measureOverlay || el === measureLabel) return;

    const rect = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    const mt = parseFloat(cs.marginTop) || 0;
    const mr = parseFloat(cs.marginRight) || 0;
    const mb = parseFloat(cs.marginBottom) || 0;
    const ml = parseFloat(cs.marginLeft) || 0;
    const bt = parseFloat(cs.borderTopWidth) || 0;
    const br = parseFloat(cs.borderRightWidth) || 0;
    const bb = parseFloat(cs.borderBottomWidth) || 0;
    const blw = parseFloat(cs.borderLeftWidth) || 0;
    const pt = parseFloat(cs.paddingTop) || 0;
    const pr = parseFloat(cs.paddingRight) || 0;
    const pb = parseFloat(cs.paddingBottom) || 0;
    const pl = parseFloat(cs.paddingLeft) || 0;

    const cw = rect.width - blw - br - pl - pr;
    const ch = rect.height - bt - bb - pt - pb;

    measureOverlay.innerHTML = ''
      // Margin
      + '<div style="position:fixed;background:rgba(255,165,0,0.3);'
      + 'left:' + (rect.left - ml) + 'px;top:' + (rect.top - mt) + 'px;'
      + 'width:' + (rect.width + ml + mr) + 'px;height:' + mt + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,165,0,0.3);'
      + 'left:' + (rect.left - ml) + 'px;top:' + (rect.bottom) + 'px;'
      + 'width:' + (rect.width + ml + mr) + 'px;height:' + mb + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,165,0,0.3);'
      + 'left:' + (rect.left - ml) + 'px;top:' + rect.top + 'px;'
      + 'width:' + ml + 'px;height:' + rect.height + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,165,0,0.3);'
      + 'left:' + rect.right + 'px;top:' + rect.top + 'px;'
      + 'width:' + mr + 'px;height:' + rect.height + 'px;"></div>'
      // Border
      + '<div style="position:fixed;background:rgba(255,255,0,0.3);'
      + 'left:' + rect.left + 'px;top:' + rect.top + 'px;'
      + 'width:' + rect.width + 'px;height:' + bt + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,255,0,0.3);'
      + 'left:' + rect.left + 'px;top:' + (rect.bottom - bb) + 'px;'
      + 'width:' + rect.width + 'px;height:' + bb + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,255,0,0.3);'
      + 'left:' + rect.left + 'px;top:' + (rect.top + bt) + 'px;'
      + 'width:' + blw + 'px;height:' + (rect.height - bt - bb) + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,255,0,0.3);'
      + 'left:' + (rect.right - br) + 'px;top:' + (rect.top + bt) + 'px;'
      + 'width:' + br + 'px;height:' + (rect.height - bt - bb) + 'px;"></div>'
      // Padding
      + '<div style="position:fixed;background:rgba(144,238,144,0.3);'
      + 'left:' + (rect.left + blw) + 'px;top:' + (rect.top + bt) + 'px;'
      + 'width:' + (rect.width - blw - br) + 'px;height:' + pt + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(144,238,144,0.3);'
      + 'left:' + (rect.left + blw) + 'px;top:' + (rect.bottom - bb - pb) + 'px;'
      + 'width:' + (rect.width - blw - br) + 'px;height:' + pb + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(144,238,144,0.3);'
      + 'left:' + (rect.left + blw) + 'px;top:' + (rect.top + bt + pt) + 'px;'
      + 'width:' + pl + 'px;height:' + (rect.height - bt - bb - pt - pb) + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(144,238,144,0.3);'
      + 'left:' + (rect.right - br - pr) + 'px;top:' + (rect.top + bt + pt) + 'px;'
      + 'width:' + pr + 'px;height:' + (rect.height - bt - bb - pt - pb) + 'px;"></div>'
      // Content
      + '<div style="position:fixed;background:rgba(100,149,237,0.3);'
      + 'left:' + (rect.left + blw + pl) + 'px;top:' + (rect.top + bt + pt) + 'px;'
      + 'width:' + cw + 'px;height:' + ch + 'px;"></div>';

    // Label
    measureLabel.textContent = Math.round(rect.width) + ' x ' + Math.round(rect.height);
    measureLabel.style.display = 'block';
    measureLabel.style.left = (rect.right + 8) + 'px';
    measureLabel.style.top = rect.top + 'px';
  }

  window.addEventListener('message', (e) => {
    if (!e.data?.type?.startsWith('musea:')) return;
    const { type, payload } = e.data;
    switch (type) {
      case 'musea:set-background': {
        if (payload.pattern === 'checkerboard') {
          document.body.style.background = '';
          document.body.classList.add('musea-bg-checkerboard');
        } else {
          document.body.classList.remove('musea-bg-checkerboard');
          document.body.style.background = payload.color || '';
        }
        break;
      }
      case 'musea:toggle-outline': {
        toggleStyleById('musea-outline', payload.enabled,
          '* { outline: 1px solid rgba(255, 0, 0, 0.3) !important; }');
        break;
      }
      case 'musea:toggle-measure': {
        measureActive = payload.enabled;
        if (measureActive) {
          createMeasureOverlay();
          document.addEventListener('mousemove', onMeasureMouseMove);
        } else {
          document.removeEventListener('mousemove', onMeasureMouseMove);
          removeMeasureOverlay();
        }
        break;
      }
      case 'musea:set-props': {
        // Store props for remount - handled by preview module
        if (window.__museaSetProps) {
          window.__museaSetProps(payload.props || {});
        }
        break;
      }
      case 'musea:set-slots': {
        // Store slots for remount - handled by preview module
        if (window.__museaSetSlots) {
          window.__museaSetSlots(payload.slots || {});
        }
        break;
      }
      case 'musea:run-a11y': {
        // Run axe-core a11y test
        (async () => {
          try {
            // Dynamically load axe-core from local vendor route if not already loaded
            if (!window.axe) {
              const script = document.createElement('script');
              const _basePath = location.pathname.replace(/\\/preview$/, '');
              script.src = _basePath + '/vendor/axe-core.min.js';
              await new Promise((resolve, reject) => {
                script.onload = resolve;
                script.onerror = reject;
                document.head.appendChild(script);
              });
            }
            // Run axe-core on the .musea-variant container only (not the full document)
            const context = document.querySelector('.musea-variant') || document;
            const results = await window.axe.run(context, {
              // Run all rules without restrictions for comprehensive testing
              resultTypes: ['violations', 'incomplete', 'passes']
            });
            window.parent.postMessage({
              type: 'musea:a11y-result',
              payload: {
                violations: results.violations.map(v => ({
                  id: v.id,
                  impact: v.impact,
                  description: v.description,
                  helpUrl: v.helpUrl,
                  nodes: v.nodes.map(n => ({
                    html: n.html,
                    target: n.target,
                    failureSummary: n.failureSummary
                  }))
                })),
                passes: results.passes.length,
                incomplete: results.incomplete.length
              }
            }, '*');
          } catch (err) {
            window.parent.postMessage({
              type: 'musea:a11y-result',
              payload: {
                error: err instanceof Error ? err.message : String(err),
                violations: [],
                passes: 0,
                incomplete: 0
              }
            }, '*');
          }
        })();
        break;
      }
    }
  });

  // Notify parent that iframe is ready
  window.parent.postMessage({ type: 'musea:ready', payload: {} }, '*');
}
`;

export function generatePreviewModule(
  art: ArtFileInfo,
  variantComponentName: string,
  variantName: string,
  cssImports: string[] = [],
  previewSetup: string | null = null,
): string {
  const artModuleId = `virtual:musea-art:${art.path}`;
  const escapedVariantName = escapeTemplate(variantName);
  const cssImportStatements = cssImports.map((cssPath) => `import '${cssPath}';`).join("\n");
  const setupImport = previewSetup ? `import __museaPreviewSetup from '${previewSetup}';` : "";
  const setupCall = previewSetup ? "await __museaPreviewSetup(app);" : "";

  return `
${cssImportStatements}
${setupImport}
import { createApp, reactive, h } from 'vue';
import * as artModule from '${artModuleId}';

const container = document.getElementById('app');

${MUSEA_ADDONS_INIT_CODE}

let currentApp = null;
const propsOverride = reactive({});
const slotsOverride = reactive({ default: '' });

window.__museaSetProps = (props) => {
  // Clear old keys
  for (const key of Object.keys(propsOverride)) {
    delete propsOverride[key];
  }
  Object.assign(propsOverride, props);
};

window.__museaSetSlots = (slots) => {
  for (const key of Object.keys(slotsOverride)) {
    delete slotsOverride[key];
  }
  Object.assign(slotsOverride, slots);
};

async function mount() {
  try {
    // Get the specific variant component
    const VariantComponent = artModule['${variantComponentName}'];
    const RawComponent = artModule.__component__;

    if (!VariantComponent) {
      throw new Error('Variant component "${variantComponentName}" not found in art module');
    }

    // Create and mount the app
    const app = createApp(VariantComponent);
    ${setupCall}
    container.innerHTML = '';
    container.className = 'musea-variant';
    app.mount(container);
    currentApp = app;

    console.log('[musea-preview] Mounted variant: ${escapedVariantName}');
    __museaInitAddons(container, '${escapedVariantName}');

    // Override set-props to remount with raw component + props
    const TargetComponent = RawComponent || VariantComponent;
    window.__museaSetProps = (props) => {
      for (const key of Object.keys(propsOverride)) {
        delete propsOverride[key];
      }
      Object.assign(propsOverride, props);
      remountWithProps(TargetComponent);
    };
    window.__museaSetSlots = (slots) => {
      for (const key of Object.keys(slotsOverride)) {
        delete slotsOverride[key];
      }
      Object.assign(slotsOverride, slots);
      remountWithProps(TargetComponent);
    };
  } catch (error) {
    console.error('[musea-preview] Failed to mount:', error);
    container.innerHTML = \`
      <div class="musea-error">
        <div class="musea-error-title">Failed to render component</div>
        <div>\${error.message}</div>
        <pre>\${error.stack || ''}</pre>
      </div>
    \`;
  }
}

async function remountWithProps(Component) {
  if (currentApp) {
    currentApp.unmount();
  }
  const app = createApp({
    setup() {
      return () => {
        const slotFns = {};
        for (const [name, content] of Object.entries(slotsOverride)) {
          if (content) slotFns[name] = () => h('span', { innerHTML: content });
        }
        return h(Component, { ...propsOverride }, slotFns);
      };
    }
  });
  ${setupCall}
  container.innerHTML = '';
  app.mount(container);
  currentApp = app;
}

mount();
`;
}

export function generatePreviewModuleWithProps(
  art: ArtFileInfo,
  variantComponentName: string,
  variantName: string,
  propsOverride: Record<string, unknown>,
  cssImports: string[] = [],
  previewSetup: string | null = null,
): string {
  const artModuleId = `virtual:musea-art:${art.path}`;
  const escapedVariantName = escapeTemplate(variantName);
  const propsJson = JSON.stringify(propsOverride);
  const cssImportStatements = cssImports.map((cssPath) => `import '${cssPath}';`).join("\n");
  const setupImport = previewSetup ? `import __museaPreviewSetup from '${previewSetup}';` : "";
  const setupCall = previewSetup ? "await __museaPreviewSetup(app);" : "";

  return `
${cssImportStatements}
${setupImport}
import { createApp, h } from 'vue';
import * as artModule from '${artModuleId}';

const container = document.getElementById('app');
const propsOverride = ${propsJson};

${MUSEA_ADDONS_INIT_CODE}

async function mount() {
  try {
    const VariantComponent = artModule['${variantComponentName}'];
    if (!VariantComponent) {
      throw new Error('Variant component "${variantComponentName}" not found');
    }

    const WrappedComponent = {
      render() {
        return h(VariantComponent, propsOverride);
      }
    };

    const app = createApp(WrappedComponent);
    ${setupCall}
    container.innerHTML = '';
    container.className = 'musea-variant';
    app.mount(container);
    console.log('[musea-preview] Mounted variant: ${escapedVariantName} with props override');
    __museaInitAddons(container, '${escapedVariantName}');
  } catch (error) {
    console.error('[musea-preview] Failed to mount:', error);
    container.innerHTML = '<div class="musea-error"><div class="musea-error-title">Failed to render</div><div>' + error.message + '</div></div>';
  }
}

mount();
`;
}

export function generatePreviewHtml(
  art: ArtFileInfo,
  variant: ArtVariant,
  _basePath: string,
  viteBase?: string,
): string {
  // Use preview-module HTTP endpoint instead of virtual module import.
  // Virtual module imports in inline scripts require transformIndexHtml,
  // which creates malformed html-proxy URLs when the page URL has query params.
  const previewModuleUrl = `${_basePath}/preview-module?art=${encodeURIComponent(art.path)}&variant=${encodeURIComponent(variant.name)}`;
  const base = (viteBase || "/").replace(/\/$/, "");

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${escapeHtml(art.metadata.title)} - ${escapeHtml(variant.name)}</title>
  <script type="module" src="${base}/@vite/client"></script>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    html, body {
      width: 100%;
      height: 100%;
    }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #ffffff;
    }
    .musea-variant {
      min-height: 100vh;
    }
    .musea-error {
      color: #dc2626;
      background: #fef2f2;
      border: 1px solid #fecaca;
      border-radius: 8px;
      padding: 1rem;
      font-size: 0.875rem;
      max-width: 400px;
    }
    .musea-error-title {
      font-weight: 600;
      margin-bottom: 0.5rem;
    }
    .musea-error pre {
      font-family: monospace;
      font-size: 0.75rem;
      white-space: pre-wrap;
      word-break: break-all;
      margin-top: 0.5rem;
      padding: 0.5rem;
      background: #fff;
      border-radius: 4px;
    }
    .musea-loading {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      color: #6b7280;
      font-size: 0.875rem;
    }
    .musea-spinner {
      width: 20px;
      height: 20px;
      border: 2px solid #e5e7eb;
      border-top-color: #3b82f6;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }
    @keyframes spin { to { transform: rotate(360deg); } }

    /* Musea Addons: Checkerboard background for transparent mode */
    .musea-bg-checkerboard {
      background-image:
        linear-gradient(45deg, #ccc 25%, transparent 25%),
        linear-gradient(-45deg, #ccc 25%, transparent 25%),
        linear-gradient(45deg, transparent 75%, #ccc 75%),
        linear-gradient(-45deg, transparent 75%, #ccc 75%) !important;
      background-size: 20px 20px !important;
      background-position: 0 0, 0 10px, 10px -10px, -10px 0 !important;
    }

    /* Musea Addons: Measure label */
    .musea-measure-label {
      position: fixed;
      background: #333;
      color: #fff;
      font-size: 11px;
      padding: 2px 6px;
      border-radius: 3px;
      pointer-events: none;
      z-index: 100000;
    }
  </style>
</head>
<body>
  <div id="app" class="musea-variant" data-art="${escapeHtml(art.path)}" data-variant="${escapeHtml(variant.name)}">
    <div class="musea-loading">
      <div class="musea-spinner"></div>
      Loading component...
    </div>
  </div>
  <script type="module" src="${previewModuleUrl}"></script>
</body>
</html>`;
}
