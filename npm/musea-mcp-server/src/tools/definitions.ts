/**
 * MCP tool definitions for Musea.
 *
 * Declares the schema (name, description, input parameters) for each tool
 * exposed by the MCP server: component analysis, registry, code generation,
 * documentation, and design tokens.
 */

export const toolDefinitions = [
  // --- Component analysis ---------------------------------------------------
  {
    name: "analyze_component",
    description:
      "Statically analyze a Vue SFC to extract its props and emits. Useful for understanding a component's public API when building or reviewing a design system.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: {
          type: "string",
          description: "Path to the .vue component file (relative to project root)",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "get_palette",
    description:
      "Derive an interactive props palette (control types, defaults, ranges, options) for a component described by an Art file. Helps to understand how props can be tweaked in a design system playground.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: {
          type: "string",
          description: "Path to the .art.vue file (relative to project root)",
        },
      },
      required: ["path"],
    },
  },

  // --- Component registry ---------------------------------------------------
  {
    name: "list_components",
    description:
      "List components registered in the design system. Returns titles, categories, tags, and variant counts.",
    inputSchema: {
      type: "object" as const,
      properties: {
        category: { type: "string", description: "Filter by category" },
        tag: { type: "string", description: "Filter by tag" },
      },
    },
  },
  {
    name: "get_component",
    description:
      "Get full details of a design-system component: metadata, variant list, and script/style information.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: {
          type: "string",
          description: "Path to the .art.vue file (relative to project root)",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "get_variant",
    description: "Retrieve a single variant (template and metadata) from a component.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: { type: "string", description: "Path to the .art.vue file" },
        variant: { type: "string", description: "Variant name" },
      },
      required: ["path", "variant"],
    },
  },
  {
    name: "search_components",
    description: "Full-text search over component titles, descriptions, and tags.",
    inputSchema: {
      type: "object" as const,
      properties: {
        query: { type: "string", description: "Search query" },
      },
      required: ["query"],
    },
  },

  // --- Code generation ------------------------------------------------------
  {
    name: "generate_variants",
    description:
      "Analyze a Vue component's props and auto-generate an .art.vue file containing appropriate variant combinations (default, boolean toggles, enum values, etc.).",
    inputSchema: {
      type: "object" as const,
      properties: {
        componentPath: {
          type: "string",
          description: "Path to the .vue component file (relative to project root)",
        },
        maxVariants: {
          type: "number",
          description: "Maximum number of variants to generate (default: 20)",
        },
        includeDefault: {
          type: "boolean",
          description: "Include a default variant (default: true)",
        },
        includeBooleanToggles: {
          type: "boolean",
          description: "Generate variants that toggle each boolean prop (default: true)",
        },
        includeEnumVariants: {
          type: "boolean",
          description: "Generate one variant per enum/union value (default: true)",
        },
      },
      required: ["componentPath"],
    },
  },
  {
    name: "generate_csf",
    description:
      "Convert an .art.vue file into Storybook CSF 3.0 code for integration with existing Storybook setups.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: { type: "string", description: "Path to the .art.vue file" },
      },
      required: ["path"],
    },
  },

  // --- Documentation --------------------------------------------------------
  {
    name: "generate_docs",
    description:
      "Generate Markdown documentation for a design-system component from its .art.vue definition.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: {
          type: "string",
          description: "Path to the .art.vue file (relative to project root)",
        },
        includeSource: {
          type: "boolean",
          description: "Embed source code in the output (default: false)",
        },
        includeTemplates: {
          type: "boolean",
          description: "Embed variant templates in the output (default: false)",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "generate_catalog",
    description:
      "Produce a single Markdown catalog covering every component in the design system, grouped by category.",
    inputSchema: {
      type: "object" as const,
      properties: {
        includeSource: {
          type: "boolean",
          description: "Embed source code in the catalog (default: false)",
        },
        includeTemplates: {
          type: "boolean",
          description: "Embed variant templates in the catalog (default: false)",
        },
      },
    },
  },

  // --- Design tokens --------------------------------------------------------
  {
    name: "get_tokens",
    description:
      "Read design tokens (colors, spacing, typography, etc.) from a Style Dictionary\u2013compatible JSON file or directory. Auto-detects common paths if not specified.",
    inputSchema: {
      type: "object" as const,
      properties: {
        tokensPath: {
          type: "string",
          description:
            "Path to tokens JSON file or directory (relative to project root). Auto-detects tokens/, design-tokens/, or style-dictionary/ if omitted.",
        },
        format: {
          type: "string",
          enum: ["json", "markdown"],
          description: "Output format (default: json)",
        },
      },
    },
  },
];
