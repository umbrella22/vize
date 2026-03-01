/**
 * Gallery HTML generation for the Musea component gallery.
 *
 * Contains the inline gallery SPA template (used as a fallback when the
 * pre-built gallery is not available) and the gallery virtual module.
 */

/**
 * Generate the inline gallery HTML page.
 */
export function generateGalleryHtml(
  basePath: string,
  themeConfig?: { default: string; custom?: Record<string, unknown> },
): string {
  const themeScript = themeConfig
    ? `window.__MUSEA_THEME_CONFIG__=${JSON.stringify(themeConfig)};`
    : "";
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Musea - Component Gallery</title>
  <script>window.__MUSEA_BASE_PATH__='${basePath}';${themeScript}${"<"}/script>
  <style>
    :root {
      --musea-bg-primary: #E6E2D6;
      --musea-bg-secondary: #ddd9cd;
      --musea-bg-tertiary: #d4d0c4;
      --musea-bg-elevated: #E6E2D6;
      --musea-accent: #121212;
      --musea-accent-hover: #2a2a2a;
      --musea-accent-subtle: rgba(18, 18, 18, 0.08);
      --musea-text: #121212;
      --musea-text-secondary: #3a3a3a;
      --musea-text-muted: #6b6b6b;
      --musea-border: #c8c4b8;
      --musea-border-subtle: #d4d0c4;
      --musea-success: #16a34a;
      --musea-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
      --musea-radius-sm: 4px;
      --musea-radius-md: 6px;
      --musea-radius-lg: 8px;
      --musea-transition: 0.15s ease;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif;
      background: var(--musea-bg-primary);
      color: var(--musea-text);
      min-height: 100vh;
      line-height: 1.5;
      -webkit-font-smoothing: antialiased;
    }

    /* Header */
    .header {
      background: var(--musea-bg-secondary);
      border-bottom: 1px solid var(--musea-border);
      padding: 0 1.5rem;
      height: 56px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      position: sticky;
      top: 0;
      z-index: 100;
    }

    .header-left {
      display: flex;
      align-items: center;
      gap: 1.5rem;
    }

    .logo {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 1.125rem;
      font-weight: 700;
      color: var(--musea-accent);
      text-decoration: none;
    }

    .logo-svg {
      width: 32px;
      height: 32px;
      flex-shrink: 0;
    }

    .logo-icon svg {
      width: 16px;
      height: 16px;
      color: var(--musea-text);
    }

    .header-subtitle {
      color: var(--musea-text-muted);
      font-size: 0.8125rem;
      font-weight: 500;
      padding-left: 1.5rem;
      border-left: 1px solid var(--musea-border);
    }

    .search-container {
      position: relative;
      width: 280px;
    }

    .search-input {
      width: 100%;
      background: var(--musea-bg-tertiary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-md);
      padding: 0.5rem 0.75rem 0.5rem 2.25rem;
      color: var(--musea-text);
      font-size: 0.8125rem;
      outline: none;
      transition: border-color var(--musea-transition), background var(--musea-transition);
    }

    .search-input::placeholder {
      color: var(--musea-text-muted);
    }

    .search-input:focus {
      border-color: var(--musea-accent);
      background: var(--musea-bg-elevated);
    }

    .search-icon {
      position: absolute;
      left: 0.75rem;
      top: 50%;
      transform: translateY(-50%);
      color: var(--musea-text-muted);
      pointer-events: none;
    }

    /* Layout */
    .main {
      display: grid;
      grid-template-columns: 260px 1fr;
      min-height: calc(100vh - 56px);
    }

    /* Sidebar */
    .sidebar {
      background: var(--musea-bg-secondary);
      border-right: 1px solid var(--musea-border);
      overflow-y: auto;
      overflow-x: hidden;
    }

    .sidebar::-webkit-scrollbar {
      width: 6px;
    }

    .sidebar::-webkit-scrollbar-track {
      background: transparent;
    }

    .sidebar::-webkit-scrollbar-thumb {
      background: var(--musea-border);
      border-radius: 3px;
    }

    .sidebar-section {
      padding: 0.75rem;
    }

    .category-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.625rem 0.75rem;
      font-size: 0.6875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--musea-text-muted);
      cursor: pointer;
      user-select: none;
      border-radius: var(--musea-radius-sm);
      transition: background var(--musea-transition);
    }

    .category-header:hover {
      background: var(--musea-bg-tertiary);
    }

    .category-icon {
      width: 16px;
      height: 16px;
      transition: transform var(--musea-transition);
    }

    .category-header.collapsed .category-icon {
      transform: rotate(-90deg);
    }

    .category-count {
      margin-left: auto;
      background: var(--musea-bg-tertiary);
      padding: 0.125rem 0.375rem;
      border-radius: 4px;
      font-size: 0.625rem;
    }

    .art-list {
      list-style: none;
      margin-top: 0.25rem;
    }

    .art-item {
      display: flex;
      align-items: center;
      gap: 0.625rem;
      padding: 0.5rem 0.75rem 0.5rem 1.75rem;
      border-radius: var(--musea-radius-sm);
      cursor: pointer;
      font-size: 0.8125rem;
      color: var(--musea-text-secondary);
      transition: all var(--musea-transition);
      position: relative;
    }

    .art-item::before {
      content: '';
      position: absolute;
      left: 0.75rem;
      top: 50%;
      transform: translateY(-50%);
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--musea-border);
      transition: background var(--musea-transition);
    }

    .art-item:hover {
      background: var(--musea-bg-tertiary);
      color: var(--musea-text);
    }

    .art-item:hover::before {
      background: var(--musea-text-muted);
    }

    .art-item.active {
      background: var(--musea-accent-subtle);
      color: var(--musea-accent-hover);
    }

    .art-item.active::before {
      background: var(--musea-accent);
    }

    .art-variant-count {
      margin-left: auto;
      font-size: 0.6875rem;
      color: var(--musea-text-muted);
      opacity: 0;
      transition: opacity var(--musea-transition);
    }

    .art-item:hover .art-variant-count {
      opacity: 1;
    }

    /* Content */
    .content {
      background: var(--musea-bg-primary);
      overflow-y: auto;
    }

    .content-inner {
      max-width: 1400px;
      margin: 0 auto;
      padding: 2rem;
    }

    .content-header {
      margin-bottom: 2rem;
    }

    .content-title {
      font-size: 1.5rem;
      font-weight: 700;
      margin-bottom: 0.5rem;
    }

    .content-description {
      color: var(--musea-text-muted);
      font-size: 0.9375rem;
      max-width: 600px;
    }

    .content-meta {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-top: 1rem;
    }

    .meta-tag {
      display: inline-flex;
      align-items: center;
      gap: 0.375rem;
      padding: 0.25rem 0.625rem;
      background: var(--musea-bg-secondary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-sm);
      font-size: 0.75rem;
      color: var(--musea-text-muted);
    }

    .meta-tag svg {
      width: 12px;
      height: 12px;
    }

    /* Gallery Grid */
    .gallery {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
      gap: 1.25rem;
    }

    /* Variant Card */
    .variant-card {
      background: var(--musea-bg-secondary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-lg);
      overflow: hidden;
      transition: all var(--musea-transition);
    }

    .variant-card:hover {
      border-color: var(--musea-text-muted);
      box-shadow: var(--musea-shadow);
      transform: translateY(-2px);
    }

    .variant-preview {
      aspect-ratio: 16 / 10;
      background: var(--musea-bg-tertiary);
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
      overflow: hidden;
    }

    .variant-preview iframe {
      width: 100%;
      height: 100%;
      border: none;
      background: white;
    }

    .variant-preview-placeholder {
      color: var(--musea-text-muted);
      font-size: 0.8125rem;
      text-align: center;
      padding: 1rem;
    }

    .variant-preview-code {
      font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
      font-size: 0.75rem;
      color: var(--musea-text-muted);
      background: var(--musea-bg-primary);
      padding: 1rem;
      overflow: auto;
      max-height: 100%;
      width: 100%;
    }

    .variant-info {
      padding: 1rem;
      border-top: 1px solid var(--musea-border);
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .variant-name {
      font-weight: 600;
      font-size: 0.875rem;
    }

    .variant-badge {
      font-size: 0.625rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      padding: 0.1875rem 0.5rem;
      border-radius: 4px;
      background: var(--musea-accent-subtle);
      color: var(--musea-accent);
    }

    .variant-actions {
      display: flex;
      gap: 0.5rem;
    }

    .variant-action-btn {
      width: 28px;
      height: 28px;
      border: none;
      background: var(--musea-bg-tertiary);
      border-radius: var(--musea-radius-sm);
      color: var(--musea-text-muted);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all var(--musea-transition);
    }

    .variant-action-btn:hover {
      background: var(--musea-bg-elevated);
      color: var(--musea-text);
    }

    .variant-action-btn svg {
      width: 14px;
      height: 14px;
    }

    /* Empty State */
    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      min-height: 400px;
      text-align: center;
      padding: 2rem;
    }

    .empty-state-icon {
      width: 80px;
      height: 80px;
      background: var(--musea-bg-secondary);
      border-radius: var(--musea-radius-lg);
      display: flex;
      align-items: center;
      justify-content: center;
      margin-bottom: 1.5rem;
    }

    .empty-state-icon svg {
      width: 40px;
      height: 40px;
      color: var(--musea-text-muted);
    }

    .empty-state-title {
      font-size: 1.125rem;
      font-weight: 600;
      margin-bottom: 0.5rem;
    }

    .empty-state-text {
      color: var(--musea-text-muted);
      font-size: 0.875rem;
      max-width: 300px;
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 200px;
      color: var(--musea-text-muted);
      gap: 0.75rem;
    }

    .loading-spinner {
      width: 20px;
      height: 20px;
      border: 2px solid var(--musea-border);
      border-top-color: var(--musea-accent);
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    /* Responsive */
    @media (max-width: 768px) {
      .main {
        grid-template-columns: 1fr;
      }
      .sidebar {
        display: none;
      }
      .header-subtitle {
        display: none;
      }
    }
  </style>
</head>
<body>
  <header class="header">
    <div class="header-left">
      <a href="${basePath}" class="logo">
        <svg class="logo-svg" width="32" height="32" viewBox="0 0 200 200" fill="none">
          <g transform="translate(30, 25) scale(1.2)">
            <g transform="translate(15, 10) skewX(-15)">
              <path d="M 65 0 L 40 60 L 70 20 L 65 0 Z" fill="currentColor"/>
              <path d="M 20 0 L 40 60 L 53 13 L 20 0 Z" fill="currentColor"/>
            </g>
          </g>
          <g transform="translate(110, 120)">
            <line x1="5" y1="10" x2="5" y2="50" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>
            <line x1="60" y1="10" x2="60" y2="50" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>
            <path d="M 0 10 L 32.5 0 L 65 10" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
            <rect x="15" y="18" width="14" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.7"/>
            <rect x="36" y="18" width="14" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.7"/>
            <rect x="23" y="35" width="18" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.6"/>
          </g>
        </svg>
        Musea
      </a>
      <span class="header-subtitle">Component Gallery</span>
    </div>
    <div class="search-container">
      <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
      </svg>
      <input type="text" class="search-input" placeholder="Search components..." id="search">
    </div>
  </header>

  <main class="main">
    <aside class="sidebar" id="sidebar">
      <div class="loading">
        <div class="loading-spinner"></div>
        Loading...
      </div>
    </aside>
    <section class="content" id="content">
      <div class="empty-state">
        <div class="empty-state-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M4 5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5Z"/>
            <path d="M4 13a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-6Z"/>
            <path d="M16 13a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1v-6Z"/>
          </svg>
        </div>
        <div class="empty-state-title">Select a component</div>
        <div class="empty-state-text">Choose a component from the sidebar to view its variants and documentation</div>
      </div>
    </section>
  </main>

  <script type="module">
    const basePath = '${basePath}';
    let arts = [];
    let selectedArt = null;
    let searchQuery = '';

    async function loadArts() {
      try {
        const res = await fetch(basePath + '/api/arts');
        arts = await res.json();
        renderSidebar();
      } catch (e) {
        console.error('Failed to load arts:', e);
        document.getElementById('sidebar').innerHTML = '<div class="loading">Failed to load</div>';
      }
    }

    function renderSidebar() {
      const sidebar = document.getElementById('sidebar');
      const categories = {};

      const filtered = searchQuery
        ? arts.filter(a => a.metadata.title.toLowerCase().includes(searchQuery.toLowerCase()))
        : arts;

      for (const art of filtered) {
        const cat = art.metadata.category || 'Components';
        if (!categories[cat]) categories[cat] = [];
        categories[cat].push(art);
      }

      if (Object.keys(categories).length === 0) {
        sidebar.innerHTML = '<div class="sidebar-section"><div class="loading">No components found</div></div>';
        return;
      }

      let html = '';
      for (const [category, items] of Object.entries(categories)) {
        html += '<div class="sidebar-section">';
        html += '<div class="category-header" data-category="' + category + '">';
        html += '<svg class="category-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m9 18 6-6-6-6"/></svg>';
        html += '<span>' + category + '</span>';
        html += '<span class="category-count">' + items.length + '</span>';
        html += '</div>';
        html += '<ul class="art-list" data-category="' + category + '">';
        for (const art of items) {
          const active = selectedArt?.path === art.path ? 'active' : '';
          const variantCount = art.variants?.length || 0;
          html += '<li class="art-item ' + active + '" data-path="' + art.path + '">';
          html += '<span>' + escapeHtml(art.metadata.title) + '</span>';
          html += '<span class="art-variant-count">' + variantCount + ' variant' + (variantCount !== 1 ? 's' : '') + '</span>';
          html += '</li>';
        }
        html += '</ul>';
        html += '</div>';
      }

      sidebar.innerHTML = html;

      sidebar.querySelectorAll('.art-item').forEach(item => {
        item.addEventListener('click', () => {
          const artPath = item.dataset.path;
          selectedArt = arts.find(a => a.path === artPath);
          renderSidebar();
          renderContent();
        });
      });

      sidebar.querySelectorAll('.category-header').forEach(header => {
        header.addEventListener('click', () => {
          header.classList.toggle('collapsed');
          const list = sidebar.querySelector('.art-list[data-category="' + header.dataset.category + '"]');
          if (list) list.style.display = header.classList.contains('collapsed') ? 'none' : 'block';
        });
      });
    }

    function renderContent() {
      const content = document.getElementById('content');
      if (!selectedArt) {
        content.innerHTML = \`
          <div class="empty-state">
            <div class="empty-state-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M4 5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5Z"/>
                <path d="M4 13a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-6Z"/>
                <path d="M16 13a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1v-6Z"/>
              </svg>
            </div>
            <div class="empty-state-title">Select a component</div>
            <div class="empty-state-text">Choose a component from the sidebar to view its variants</div>
          </div>
        \`;
        return;
      }

      const meta = selectedArt.metadata;
      const tags = meta.tags || [];
      const variantCount = selectedArt.variants?.length || 0;

      let html = '<div class="content-inner">';
      html += '<div class="content-header">';
      html += '<h1 class="content-title">' + escapeHtml(meta.title) + '</h1>';
      if (meta.description) {
        html += '<p class="content-description">' + escapeHtml(meta.description) + '</p>';
      }
      html += '<div class="content-meta">';
      html += '<span class="meta-tag"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></svg>' + variantCount + ' variant' + (variantCount !== 1 ? 's' : '') + '</span>';
      if (meta.category) {
        html += '<span class="meta-tag"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>' + escapeHtml(meta.category) + '</span>';
      }
      for (const tag of tags) {
        html += '<span class="meta-tag">#' + escapeHtml(tag) + '</span>';
      }
      html += '</div>';
      html += '</div>';

      html += '<div class="gallery">';
      for (const variant of selectedArt.variants) {
        const previewUrl = basePath + '/preview?art=' + encodeURIComponent(selectedArt.path) + '&variant=' + encodeURIComponent(variant.name);

        html += '<div class="variant-card">';
        html += '<div class="variant-preview">';
        html += '<iframe src="' + previewUrl + '" loading="lazy" title="' + escapeHtml(variant.name) + '"></iframe>';
        html += '</div>';
        html += '<div class="variant-info">';
        html += '<div>';
        html += '<span class="variant-name">' + escapeHtml(variant.name) + '</span>';
        if (variant.isDefault) html += ' <span class="variant-badge">Default</span>';
        html += '</div>';
        html += '<div class="variant-actions">';
        html += '<button class="variant-action-btn" title="Open in new tab" onclick="window.open(\\'' + previewUrl + '\\', \\'_blank\\')"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg></button>';
        html += '</div>';
        html += '</div>';
        html += '</div>';
      }
      html += '</div>';
      html += '</div>';

      content.innerHTML = html;
    }

    function escapeHtml(str) {
      if (!str) return '';
      return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    // Search
    document.getElementById('search').addEventListener('input', (e) => {
      searchQuery = e.target.value;
      renderSidebar();
    });

    // Keyboard shortcut for search
    document.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        document.getElementById('search').focus();
      }
    });

    loadArts();
  </script>
</body>
</html>`;
}

/**
 * Generate the virtual gallery module code.
 */
export function generateGalleryModule(basePath: string): string {
  return `
export const basePath = '${basePath}';
export async function loadArts() {
  const res = await fetch(basePath + '/api/arts');
  return res.json();
}
`;
}
