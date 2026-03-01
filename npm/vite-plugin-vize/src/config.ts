/**
 * Vize configuration loading and management.
 *
 * Handles discovery, loading, and caching of vize.config.ts/js/json files,
 * and provides the shared config store for inter-plugin communication.
 */

import path from "node:path";
import fs from "node:fs";

import type { VizeConfig, ConfigEnv, UserConfigExport, LoadConfigOptions } from "./types.js";

export const CONFIG_FILES = [
  "vize.config.ts",
  "vize.config.js",
  "vize.config.mjs",
  "vize.config.json",
];

const DEFAULT_CONFIG_ENV: ConfigEnv = {
  mode: "development",
  command: "serve",
};

/**
 * Define a Vize configuration with type checking.
 * Accepts a plain object or a function that receives ConfigEnv.
 */
export function defineConfig(config: UserConfigExport): UserConfigExport {
  return config;
}

/**
 * Load Vize configuration from file
 */
export async function loadConfig(
  root: string,
  options: LoadConfigOptions = {},
): Promise<VizeConfig | null> {
  const { mode = "root", configFile, env } = options;

  if (mode === "none") return null;

  if (configFile) {
    const configPath = path.isAbsolute(configFile) ? configFile : path.resolve(root, configFile);
    return loadConfigFile(configPath, env);
  }

  if (mode === "auto") {
    let searchDir = root;
    while (true) {
      const found = findConfigInDir(searchDir);
      if (found) return loadConfigFile(found, env);
      const parentDir = path.dirname(searchDir);
      if (parentDir === searchDir) break;
      searchDir = parentDir;
    }
    return null;
  }

  // mode === "root"
  const found = findConfigInDir(root);
  return found ? loadConfigFile(found, env) : null;
}

function findConfigInDir(dir: string): string | null {
  for (const filename of CONFIG_FILES) {
    const configPath = path.join(dir, filename);
    if (fs.existsSync(configPath)) return configPath;
  }
  return null;
}

async function resolveConfigExport(
  exported: UserConfigExport,
  env?: ConfigEnv,
): Promise<VizeConfig> {
  if (typeof exported === "function") {
    return exported(env ?? DEFAULT_CONFIG_ENV);
  }
  return exported;
}

async function loadConfigFile(configPath: string, env?: ConfigEnv): Promise<VizeConfig | null> {
  if (!fs.existsSync(configPath)) return null;

  const ext = path.extname(configPath);

  if (ext === ".json") {
    const content = fs.readFileSync(configPath, "utf-8");
    return JSON.parse(content) as VizeConfig;
  }

  try {
    const module = await import(configPath);
    const exported: UserConfigExport = module.default ?? module;
    return resolveConfigExport(exported, env);
  } catch (e) {
    console.warn(`[vize] Failed to load config from ${configPath}:`, e);
    return null;
  }
}

/**
 * Shared config store for inter-plugin communication.
 * Key = project root, Value = resolved VizeConfig.
 * Used by musea() and other plugins to access the unified config.
 */
export const vizeConfigStore = new Map<string, VizeConfig>();
