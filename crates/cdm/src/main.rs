// src/main.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "cdm")]
#[command(about = "CLI for contextual data modeling", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a CDM file
    Validate {
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
    /// Build output files from a CDM schema using configured plugins
    Build {
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
    /// Generate migration files from schema changes
    Migrate {
        /// Path to CDM file to migrate (exactly one required)
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Migration name (required)
        #[arg(short = 'n', long)]
        name: String,

        /// Override migrations output directory
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,

        /// Show deltas without generating files
        #[arg(long)]
        dry_run: bool,
    },
    /// Plugin management commands
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// Create a new plugin from a template
    New {
        /// Name of the plugin to create
        #[arg(value_name = "NAME")]
        name: String,

        /// Programming language for the plugin
        #[arg(short = 'l', long, value_name = "LANG")]
        lang: String,

        /// Output directory for the plugin (defaults to current directory)
        #[arg(short = 'o', long, value_name = "DIR")]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { path } => {
            let tree = match cdm::FileResolver::load(&path) {
                Ok(tree) => tree,
                Err(diagnostics) => {
                    for diagnostic in &diagnostics {
                        eprintln!("{}", diagnostic);
                    }
                    std::process::exit(1);
                }
            };

            match cdm::validate_tree(tree) {
                Ok(result) => {
                    for diagnostic in &result.diagnostics {
                        if diagnostic.severity == cdm::Severity::Error {
                            eprintln!("{}", diagnostic);
                        } else {
                            println!("{}", diagnostic);
                        }
                    }
                }
                Err(diagnostics) => {
                    for diagnostic in &diagnostics {
                        eprintln!("{}", diagnostic);
                    }
                    std::process::exit(1);
                }
            }
        }
        Commands::Build { path } => {
            if let Err(err) = cdm::build(&path) {
                eprintln!("Build failed: {}", err);
                std::process::exit(1);
            }
        }
        Commands::Migrate { file, name, output, dry_run } => {
            if let Err(err) = cdm::migrate(&file, name, output, dry_run) {
                eprintln!("Migrate failed: {}", err);
                std::process::exit(1);
            }
        }
        Commands::Plugin { command } => {
            match command {
                PluginCommands::New { name, lang, output } => {
                    if let Err(err) = cdm::plugin_new(&name, &lang, output.as_deref()) {
                        eprintln!("Failed to create plugin: {}", err);
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    Ok(())
}