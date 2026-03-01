//! Musea command - Component gallery server

use clap::{Args, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct MuseaArgs {
    #[command(subcommand)]
    pub command: Option<MuseaCommand>,
}

#[derive(Subcommand)]
pub enum MuseaCommand {
    /// Start the component gallery server (default)
    Serve(ServeArgs),

    /// Create a new story project
    New(NewArgs),
}

#[derive(Args, Default)]
#[allow(clippy::disallowed_types)]
pub struct ServeArgs {
    /// Port to run the server on
    #[arg(short, long, default_value = "6006")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "localhost")]
    pub host: String,

    /// Stories directory
    #[arg(short, long)]
    pub stories: Option<PathBuf>,

    /// Open browser automatically
    #[arg(long)]
    pub open: bool,
}

#[derive(Args)]
#[allow(clippy::disallowed_types)]
pub struct NewArgs {
    /// Name of the story project (defaults to current directory name)
    pub name: Option<String>,

    /// Directory to create the project in (defaults to current directory)
    #[arg(short, long)]
    pub path: Option<PathBuf>,
}

pub fn run(args: MuseaArgs) {
    match args.command {
        Some(MuseaCommand::Serve(serve_args)) => run_serve(serve_args),
        Some(MuseaCommand::New(new_args)) => run_new(new_args),
        None => {
            // Default to serve
            run_serve(ServeArgs::default());
        }
    }
}

fn run_serve(args: ServeArgs) {
    eprintln!("vize musea: Starting component gallery...");
    eprintln!("  host: {}", args.host);
    eprintln!("  port: {}", args.port);
    eprintln!("  open: {}", args.open);

    vize_musea::serve();
}

fn run_new(args: NewArgs) {
    let target_dir = args.path.unwrap_or_else(|| PathBuf::from("."));
    #[allow(clippy::disallowed_types, clippy::disallowed_methods)]
    let project_name = args.name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "stories".to_string())
    });

    eprintln!(
        "vize musea new: Creating story project '{}'...",
        project_name
    );

    // Create stories directory structure
    let stories_dir = target_dir.join("stories");
    if let Err(e) = fs::create_dir_all(&stories_dir) {
        eprintln!("Error creating stories directory: {}", e);
        std::process::exit(1);
    }

    // Create example story file
    let example_story = stories_dir.join("Button.story.vue");
    let example_content = r#"<script setup lang="ts">
import { ref } from 'vue'

// Story metadata
defineOptions({
  title: 'Components/Button',
})

// Story props
const label = ref('Click me')
const variant = ref<'primary' | 'secondary' | 'outline'>('primary')
const disabled = ref(false)
</script>

<template>
  <Story title="Button">
    <Variant title="Primary">
      <button class="btn btn-primary" :disabled="disabled">
        {{ label }}
      </button>
    </Variant>

    <Variant title="Secondary">
      <button class="btn btn-secondary" :disabled="disabled">
        {{ label }}
      </button>
    </Variant>

    <Variant title="Outline">
      <button class="btn btn-outline" :disabled="disabled">
        {{ label }}
      </button>
    </Variant>
  </Story>
</template>

<style scoped>
.btn {
  padding: 0.5rem 1rem;
  border-radius: 0.375rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background-color: #3b82f6;
  color: white;
  border: none;
}

.btn-primary:hover {
  background-color: #2563eb;
}

.btn-secondary {
  background-color: #6b7280;
  color: white;
  border: none;
}

.btn-secondary:hover {
  background-color: #4b5563;
}

.btn-outline {
  background-color: transparent;
  color: #3b82f6;
  border: 1px solid #3b82f6;
}

.btn-outline:hover {
  background-color: #3b82f6;
  color: white;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
"#;

    if let Err(e) = fs::write(&example_story, example_content) {
        eprintln!("Error creating example story: {}", e);
        std::process::exit(1);
    }

    // Create vize.config.ts
    let config_path = target_dir.join("vize.config.ts");
    if !config_path.exists() {
        let config_content = r#"import { defineConfig } from 'vize'

export default defineConfig({
  musea: {
    stories: ['./stories/**/*.story.vue'],
    port: 6006,
  },
})
"#;
        if let Err(e) = fs::write(&config_path, config_content) {
            eprintln!("Error creating vize.config.ts: {}", e);
            std::process::exit(1);
        }
        eprintln!("  Created vize.config.ts");
    }

    eprintln!("  Created stories/Button.story.vue");
    eprintln!();
    eprintln!("Story project '{}' created successfully!", project_name);
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  1. Run 'vize musea' to start the component gallery");
    eprintln!("  2. Add more stories in the 'stories' directory");
}
