//! Cross-file analyzer implementation.

use super::super::analyzers;
use super::super::graph::{DependencyEdge, DependencyGraph, ModuleNode};
use super::super::registry::{FileId, ModuleRegistry};
use super::types::{CrossFileOptions, CrossFileResult, CrossFileStats};
use crate::{Analyzer, AnalyzerOptions, Croquis};
use std::path::Path;

/// Cross-file analyzer for Vue projects.
pub struct CrossFileAnalyzer {
    /// Analysis options.
    options: CrossFileOptions,
    /// Module registry.
    registry: ModuleRegistry,
    /// Dependency graph.
    graph: DependencyGraph,
    /// Single-file analyzer options.
    single_file_options: AnalyzerOptions,
}

impl CrossFileAnalyzer {
    /// Create a new cross-file analyzer.
    pub fn new(options: CrossFileOptions) -> Self {
        Self {
            options,
            registry: ModuleRegistry::new(),
            graph: DependencyGraph::new(),
            single_file_options: AnalyzerOptions::full(),
        }
    }

    /// Create with a project root directory.
    pub fn with_project_root(options: CrossFileOptions, root: impl AsRef<Path>) -> Self {
        Self {
            options,
            registry: ModuleRegistry::with_project_root(root.as_ref()),
            graph: DependencyGraph::new(),
            single_file_options: AnalyzerOptions::full(),
        }
    }

    /// Set single-file analyzer options.
    pub fn set_single_file_options(&mut self, options: AnalyzerOptions) {
        self.single_file_options = options;
    }

    /// Add a file to be analyzed.
    pub fn add_file(&mut self, path: impl AsRef<Path>, source: &str) -> FileId {
        let path = path.as_ref();

        // Analyze the file with single-file analyzer
        let analysis = self.analyze_single_file(source, path);

        // Register in module registry (takes ownership of analysis)
        let (file_id, is_new) = self.registry.register(path, source, analysis);

        if is_new {
            // Add to dependency graph
            let mut node = ModuleNode::new(file_id, path.to_string_lossy().as_ref());

            // Extract component name
            if let Some(entry) = self.registry.get(file_id) {
                node.component_name = entry.component_name.clone();
            }

            // Mark entry points
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if filename == "App.vue"
                || filename == "main.ts"
                || filename == "main.js"
                || filename == "index.vue"
            {
                node.is_entry = true;
            }

            self.graph.add_node(node);
        }

        // Update dependencies based on imports (get from registry)
        if let Some(entry) = self.registry.get(file_id) {
            // Collect data we need before calling update_dependencies
            let imports_data: Vec<_> = entry
                .analysis
                .scopes
                .iter()
                .filter(|s| s.kind == crate::scope::ScopeKind::ExternalModule)
                .filter_map(|s| {
                    if let crate::scope::ScopeData::ExternalModule(data) = s.data() {
                        Some((data.source.clone(), data.is_type_only))
                    } else {
                        None
                    }
                })
                .collect();

            let used_components: Vec<_> = entry.analysis.used_components.iter().cloned().collect();

            // Now update dependencies
            for (source, is_type_only) in imports_data {
                if let Some(target_id) = self.resolve_import(&source) {
                    // TODO: Distinguish type-only imports when tracking is needed
                    let edge_type = if is_type_only {
                        DependencyEdge::TypeImport
                    } else {
                        DependencyEdge::Import
                    };
                    self.graph.add_edge(file_id, target_id, edge_type);
                }
            }

            for component in used_components {
                if let Some(target_id) = self.graph.find_by_component(component.as_str()) {
                    self.graph
                        .add_edge(file_id, target_id, DependencyEdge::ComponentUsage);
                }
            }
        }

        file_id
    }

    /// Add multiple files.
    pub fn add_files(&mut self, files: &[(&Path, &str)]) {
        for (path, source) in files {
            self.add_file(path, source);
        }
    }

    /// Add a file with pre-computed analysis.
    ///
    /// This is useful when the caller has already performed analysis (e.g., WASM bindings
    /// that parse both script and template content). The analysis should include
    /// `used_components` populated from template analysis for component usage edges.
    pub fn add_file_with_analysis(
        &mut self,
        path: impl AsRef<Path>,
        source: &str,
        analysis: Croquis,
    ) -> FileId {
        let path = path.as_ref();

        // Register in module registry (takes ownership of analysis)
        let (file_id, is_new) = self.registry.register(path, source, analysis);

        if is_new {
            // Add to dependency graph
            let mut node = ModuleNode::new(file_id, path.to_string_lossy().as_ref());

            // Extract component name
            if let Some(entry) = self.registry.get(file_id) {
                node.component_name = entry.component_name.clone();
            }

            // Mark entry points
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if filename == "App.vue"
                || filename == "main.ts"
                || filename == "main.js"
                || filename == "index.vue"
            {
                node.is_entry = true;
            }

            self.graph.add_node(node);
        }

        // Update dependencies based on imports (get from registry)
        if let Some(entry) = self.registry.get(file_id) {
            // Collect data we need before calling update_dependencies
            let imports_data: Vec<_> = entry
                .analysis
                .scopes
                .iter()
                .filter(|s| s.kind == crate::scope::ScopeKind::ExternalModule)
                .filter_map(|s| {
                    if let crate::scope::ScopeData::ExternalModule(data) = s.data() {
                        Some((data.source.clone(), data.is_type_only))
                    } else {
                        None
                    }
                })
                .collect();

            let used_components: Vec<_> = entry.analysis.used_components.iter().cloned().collect();

            // Now update dependencies
            for (source, is_type_only) in imports_data {
                if let Some(target_id) = self.resolve_import(&source) {
                    let edge_type = if is_type_only {
                        DependencyEdge::TypeImport
                    } else {
                        DependencyEdge::Import
                    };
                    self.graph.add_edge(file_id, target_id, edge_type);
                }
            }

            for component in used_components {
                if let Some(target_id) = self.graph.find_by_component(component.as_str()) {
                    self.graph
                        .add_edge(file_id, target_id, DependencyEdge::ComponentUsage);
                }
            }
        }

        file_id
    }

    /// Rebuild component usage edges.
    ///
    /// This should be called after all files have been added to ensure
    /// that ComponentUsage edges are correctly established. When files
    /// are added one by one, component references might not resolve
    /// if the target component hasn't been added yet.
    pub fn rebuild_component_edges(&mut self) {
        // Collect all used_components from all files
        let component_data: Vec<_> = self
            .registry
            .iter()
            .map(|entry| {
                let components: Vec<_> = entry.analysis.used_components.iter().cloned().collect();
                (entry.id, components)
            })
            .collect();

        // Add ComponentUsage edges for any that were missed
        for (file_id, used_components) in component_data {
            for component in used_components {
                if let Some(target_id) = self.graph.find_by_component(component.as_str()) {
                    // add_edge checks for duplicates internally
                    self.graph
                        .add_edge(file_id, target_id, DependencyEdge::ComponentUsage);
                }
            }
        }
    }

    /// Run cross-file analysis.
    pub fn analyze(&mut self) -> CrossFileResult {
        // Note: std::time::Instant is not available in WASM, so we conditionally
        // compile time measurement only for non-WASM targets
        #[cfg(not(target_arch = "wasm32"))]
        let start_time = std::time::Instant::now();

        let mut result = CrossFileResult::default();

        // Detect circular dependencies first
        if self.options.circular_dependencies {
            self.graph.detect_circular_dependencies();
            result.circular_deps = self.graph.circular_dependencies().to_vec();
        }

        // Run enabled analyzers
        if self.options.fallthrough_attrs {
            let (info, diags) = analyzers::analyze_fallthrough(&self.registry, &self.graph);
            result.fallthrough_info = info;
            result.diagnostics.extend(diags);
        }

        if self.options.component_emits {
            let (flows, diags) = analyzers::analyze_emits(&self.registry, &self.graph);
            result.emit_flows = flows;
            result.diagnostics.extend(diags);
        }

        if self.options.event_bubbling {
            let (bubbles, diags) = analyzers::analyze_event_bubbling(&self.registry, &self.graph);
            result.event_bubbles = bubbles;
            result.diagnostics.extend(diags);
        }

        if self.options.provide_inject {
            let (matches, diags) = analyzers::analyze_provide_inject(&self.registry, &self.graph);
            result.provide_inject_matches = matches;
            result.diagnostics.extend(diags);
        }

        if self.options.unique_ids {
            let (issues, diags) = analyzers::analyze_element_ids(&self.registry);
            result.unique_id_issues = issues;
            result.diagnostics.extend(diags);
        }

        if self.options.server_client_boundary || self.options.error_suspense_boundary {
            let (boundaries, diags) = analyzers::analyze_boundaries(&self.registry, &self.graph);
            result.boundaries = boundaries;
            result.diagnostics.extend(diags);
        }

        if self.options.reactivity_tracking {
            // Single-file reactivity analysis
            let (issues, diags) = analyzers::analyze_reactivity(&self.registry, &self.graph);
            result.reactivity_issues = issues;
            result.diagnostics.extend(diags);

            // Cross-file reactivity analysis
            let (cross_issues, cross_diags) =
                analyzers::analyze_cross_file_reactivity(&self.registry, &self.graph);
            result.cross_file_reactivity_issues = cross_issues;
            result.diagnostics.extend(cross_diags);
        }

        if self.options.setup_context {
            // Setup context violation analysis (CSRP/memory leaks)
            let (issues, diags) = analyzers::analyze_setup_context(&self.registry, &self.graph);
            result.setup_context_issues = issues;
            result.diagnostics.extend(diags);
        }

        // Static validation analyzers
        if self.options.component_resolution {
            let (issues, diags) =
                analyzers::analyze_component_resolution(&self.registry, &self.graph);
            result.component_resolution_issues = issues;
            result.diagnostics.extend(diags);
        }

        if self.options.props_validation {
            let (issues, diags) = analyzers::analyze_props_validation(&self.registry, &self.graph);
            result.props_validation_issues = issues;
            result.diagnostics.extend(diags);
        }

        // Calculate statistics
        let error_count = result.diagnostics.iter().filter(|d| d.is_error()).count();
        let warning_count = result.diagnostics.iter().filter(|d| d.is_warning()).count();

        #[cfg(not(target_arch = "wasm32"))]
        let analysis_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let analysis_time_ms = 0.0; // Time measurement not available in WASM

        result.stats = CrossFileStats {
            files_analyzed: self.registry.len(),
            vue_components: self.registry.vue_components().count(),
            dependency_edges: self.count_edges(),
            error_count,
            warning_count,
            info_count: result.diagnostics.len() - error_count - warning_count,
            analysis_time_ms,
        };

        result
    }

    /// Get the module registry.
    #[inline]
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    /// Get the dependency graph.
    #[inline]
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Get analysis for a specific file.
    pub fn get_analysis(&self, file_id: FileId) -> Option<&Croquis> {
        self.registry.get(file_id).map(|e| &e.analysis)
    }

    /// Get file path by ID.
    pub fn get_file_path(&self, file_id: FileId) -> Option<&Path> {
        self.registry.get(file_id).map(|e| e.path.as_path())
    }

    /// Clear all data and reset.
    pub fn clear(&mut self) {
        self.registry.clear();
        self.graph = DependencyGraph::new();
    }

    // === Private methods ===

    fn analyze_single_file(&self, source: &str, path: &Path) -> Croquis {
        let mut analyzer = Analyzer::with_options(self.single_file_options);

        // Detect if it's a Vue SFC
        let is_vue = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("vue"));

        if is_vue {
            // For Vue SFC, we need the script content extracted.
            // The caller should pass just the script content, or use
            // the WASM bindings which properly parse SFC.
            // For cross-file analysis, we treat Vue SFC source as script setup.
            analyzer.analyze_script_setup(source);
        } else {
            analyzer.analyze_script_plain(source);
        }

        analyzer.finish()
    }

    fn resolve_import(&self, specifier: &str) -> Option<FileId> {
        // Simple resolution - check if we have this file in the registry
        // A full implementation would use import_resolver

        // Handle relative imports
        if specifier.starts_with('.') {
            // Would need current file context to resolve
            return None;
        }

        // Check by filename
        for entry in self.registry.iter() {
            if entry.filename.as_str() == specifier || {
                #[allow(clippy::disallowed_macros)]
                let vue_name = format!("{}.vue", specifier);
                entry.filename.as_str() == vue_name
            } {
                return Some(entry.id);
            }
        }

        None
    }

    fn count_edges(&self) -> usize {
        self.graph.nodes().map(|n| n.imports.len()).sum()
    }
}

impl Default for CrossFileAnalyzer {
    fn default() -> Self {
        Self::new(CrossFileOptions::default())
    }
}
