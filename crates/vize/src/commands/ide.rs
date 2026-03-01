//! IDE command - Editor integration and LSP server
//!
//! This command provides:
//! - LSP server (default, alias for `vize lsp`)
//! - Editor extension installation for VSCode and Zed

use clap::{Args, Subcommand};
use std::path::PathBuf;
use std::process::Command;

#[derive(Args)]
pub struct IdeArgs {
    #[command(subcommand)]
    pub command: Option<IdeCommands>,

    /// Use stdio for communication (default, when no subcommand)
    #[arg(long, default_value = "true")]
    pub stdio: bool,

    /// TCP port for socket communication
    #[arg(long)]
    pub port: Option<u16>,

    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,
}

#[derive(Subcommand)]
pub enum IdeCommands {
    /// Install or manage VSCode extension
    Vscode(EditorArgs),
    /// Install or manage Zed extension
    Zed(EditorArgs),
}

#[derive(Args)]
pub struct EditorArgs {
    /// Install the extension
    #[arg(long)]
    pub install: bool,

    /// Uninstall the extension
    #[arg(long)]
    pub uninstall: bool,

    /// Show extension status
    #[arg(long)]
    pub status: bool,
}

pub fn run(args: IdeArgs) {
    match args.command {
        Some(IdeCommands::Vscode(editor_args)) => run_vscode(editor_args),
        Some(IdeCommands::Zed(editor_args)) => run_zed(editor_args),
        None => run_lsp(args),
    }
}

/// Run LSP server (default behavior)
fn run_lsp(args: IdeArgs) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    runtime.block_on(async {
        let result = if let Some(port) = args.port {
            vize_maestro::serve_tcp(port).await
        } else {
            vize_maestro::serve().await
        };

        if let Err(e) = result {
            eprintln!("LSP server error: {}", e);
            std::process::exit(1);
        }
    });
}

/// Handle VSCode extension operations
fn run_vscode(args: EditorArgs) {
    if args.uninstall {
        vscode_uninstall();
    } else if args.status {
        vscode_status();
    } else if args.install {
        vscode_install();
    } else {
        // Default to install
        vscode_install();
    }
}

/// Handle Zed extension operations
fn run_zed(args: EditorArgs) {
    if args.uninstall {
        zed_uninstall();
    } else if args.status {
        zed_status();
    } else if args.install {
        zed_install();
    } else {
        // Default to install
        zed_install();
    }
}

// =============================================================================
// VSCode Extension
// =============================================================================

fn vscode_install() {
    println!("Installing Vize VSCode extension...");

    // Try to find the VSIX file
    let vsix_path = find_vscode_vsix();

    match vsix_path {
        Some(path) => {
            println!("Found extension: {}", path.display());
            install_vsix(&path);
        }
        None => {
            // Try to build from source
            println!("VSIX not found, building from source...");
            if build_vscode_extension() {
                if let Some(path) = find_vscode_vsix() {
                    install_vsix(&path);
                } else {
                    eprintln!("Failed to find built VSIX");
                    std::process::exit(1);
                }
            } else {
                eprintln!("Failed to build VSCode extension");
                eprintln!("Please ensure pnpm is installed and run from the vize repository");
                std::process::exit(1);
            }
        }
    }
}

fn vscode_uninstall() {
    println!("Uninstalling Vize VSCode extension...");

    let status = Command::new("code")
        .args(["--uninstall-extension", "vize.vize"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✓ Vize extension uninstalled from VSCode");
        }
        Ok(_) => {
            eprintln!("Extension not installed or already removed");
        }
        Err(e) => {
            eprintln!("Failed to run 'code' command: {}", e);
            eprintln!("Make sure VSCode is installed and 'code' is in your PATH");
            std::process::exit(1);
        }
    }
}

fn vscode_status() {
    let output = Command::new("code").args(["--list-extensions"]).output();

    match output {
        Ok(out) => {
            #[allow(clippy::disallowed_types)]
            let extensions = std::string::String::from_utf8_lossy(&out.stdout);
            if extensions.contains("vize.vize") {
                println!("✓ Vize extension is installed in VSCode");
            } else {
                println!("✗ Vize extension is not installed in VSCode");
            }
        }
        Err(e) => {
            eprintln!("Failed to check VSCode extensions: {}", e);
            eprintln!("Make sure VSCode is installed and 'code' is in your PATH");
        }
    }
}

fn find_vscode_vsix() -> Option<PathBuf> {
    // Look for VSIX in common locations
    let locations = [
        // Current working directory
        PathBuf::from("npm/vscode-vize"),
        // Relative to executable
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("../../npm/vscode-vize")))
            .unwrap_or_default(),
    ];

    for base in &locations {
        // Look for any .vsix file
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "vsix").unwrap_or(false) {
                    return Some(path);
                }
            }
        }
    }

    None
}

fn build_vscode_extension() -> bool {
    // Try to find the extension source
    let source_dir = PathBuf::from("npm/vscode-vize");
    if !source_dir.exists() {
        return false;
    }

    println!("Building VSCode extension...");

    // Run pnpm install and build
    let install_status = Command::new("pnpm")
        .args(["install"])
        .current_dir(&source_dir)
        .status();

    if !install_status.map(|s| s.success()).unwrap_or(false) {
        return false;
    }

    // Package the extension
    let package_status = Command::new("pnpm")
        .args(["run", "package"])
        .current_dir(&source_dir)
        .status();

    package_status.map(|s| s.success()).unwrap_or(false)
}

fn install_vsix(path: &std::path::Path) {
    println!("Installing VSIX: {}", path.display());

    let status = Command::new("code")
        .args(["--install-extension", &path.to_string_lossy()])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✓ Vize extension installed successfully!");
            println!("  Restart VSCode to activate the extension.");
        }
        Ok(_) => {
            eprintln!("Failed to install extension");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to run 'code' command: {}", e);
            eprintln!("Make sure VSCode is installed and 'code' is in your PATH");
            std::process::exit(1);
        }
    }
}

// =============================================================================
// Zed Extension
// =============================================================================

fn zed_install() {
    println!("Installing Vize Zed extension...");

    let extensions_dir = get_zed_extensions_dir();

    match extensions_dir {
        Some(dir) => {
            let vize_dir = dir.join("vize");

            // Create extension directory
            if let Err(e) = std::fs::create_dir_all(&vize_dir) {
                eprintln!("Failed to create extension directory: {}", e);
                std::process::exit(1);
            }

            // Create extension.toml
            let extension_toml = r#"id = "vize"
name = "Vize"
description = "Vue language support powered by Vize"
version = "0.0.1"
schema_version = 1
authors = ["Vize <https://github.com/ubugeeei/vize>"]
repository = "https://github.com/ubugeeei/vize"

[language_servers.vize]
name = "Vize Language Server"
languages = ["Vue"]
"#;

            if let Err(e) = std::fs::write(vize_dir.join("extension.toml"), extension_toml) {
                eprintln!("Failed to write extension.toml: {}", e);
                std::process::exit(1);
            }

            // Create languages directory
            let languages_dir = vize_dir.join("languages");
            if let Err(e) = std::fs::create_dir_all(&languages_dir) {
                eprintln!("Failed to create languages directory: {}", e);
                std::process::exit(1);
            }

            // Create Vue language config
            let vue_config = r#"name = "Vue"
grammar = "vue"
path_suffixes = ["vue"]
line_comments = ["// "]
block_comment = ["<!--", "-->"]
brackets = [
    { start = "{", end = "}", close = true, newline = true },
    { start = "[", end = "]", close = true, newline = true },
    { start = "(", end = ")", close = true, newline = true },
    { start = "<", end = ">", close = true, newline = true },
    { start = "\"", end = "\"", close = true, newline = false },
    { start = "'", end = "'", close = true, newline = false },
]
"#;

            if let Err(e) = std::fs::write(languages_dir.join("vue.toml"), vue_config) {
                eprintln!("Failed to write vue.toml: {}", e);
                std::process::exit(1);
            }

            println!("✓ Vize extension installed to: {}", vize_dir.display());
            println!("  Note: Configure 'vize lsp' as the language server in Zed settings.");
            println!();
            println!("  Add to your Zed settings.json:");
            println!("  {{");
            println!("    \"lsp\": {{");
            println!("      \"vize\": {{");
            println!("        \"binary\": {{");
            println!("          \"path\": \"vize\",");
            println!("          \"arguments\": [\"lsp\"]");
            println!("        }}");
            println!("      }}");
            println!("    }}");
            println!("  }}");
        }
        None => {
            eprintln!("Could not find Zed extensions directory");
            eprintln!("Make sure Zed is installed");
            std::process::exit(1);
        }
    }
}

fn zed_uninstall() {
    println!("Uninstalling Vize Zed extension...");

    let extensions_dir = get_zed_extensions_dir();

    match extensions_dir {
        Some(dir) => {
            let vize_dir = dir.join("vize");

            if vize_dir.exists() {
                if let Err(e) = std::fs::remove_dir_all(&vize_dir) {
                    eprintln!("Failed to remove extension: {}", e);
                    std::process::exit(1);
                }
                println!("✓ Vize extension removed from Zed");
            } else {
                println!("Extension not installed");
            }
        }
        None => {
            eprintln!("Could not find Zed extensions directory");
        }
    }
}

fn zed_status() {
    let extensions_dir = get_zed_extensions_dir();

    match extensions_dir {
        Some(dir) => {
            let vize_dir = dir.join("vize");
            if vize_dir.exists() {
                println!("✓ Vize extension is installed in Zed");
                println!("  Location: {}", vize_dir.display());
            } else {
                println!("✗ Vize extension is not installed in Zed");
            }
        }
        None => {
            println!("✗ Zed extensions directory not found");
        }
    }
}

fn get_zed_extensions_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join(".config/zed/extensions/installed"))
    }

    #[cfg(target_os = "linux")]
    {
        dirs::config_dir().map(|c| c.join("zed/extensions/installed"))
    }

    #[cfg(target_os = "windows")]
    {
        dirs::config_dir().map(|c| c.join("Zed/extensions/installed"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}
