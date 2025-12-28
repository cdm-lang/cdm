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

        /// Warn about entities without IDs for migration tracking
        #[arg(long)]
        check_ids: bool,
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
    /// Format CDM files and optionally assign entity IDs
    Format {
        /// Files or glob patterns to format
        #[arg(value_name = "FILES", required = true)]
        files: Vec<String>,

        /// Auto-assign entity IDs to entities without them
        #[arg(long)]
        assign_ids: bool,

        /// Check formatting without writing changes (dry-run)
        #[arg(long)]
        check: bool,

        /// Number of spaces for indentation (default: 2)
        #[arg(long, default_value = "2")]
        indent: usize,
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
    /// List available plugins from registry or cache
    List {
        /// List cached plugins instead of registry
        #[arg(long)]
        cached: bool,
    },
    /// Show information about a plugin
    Info {
        /// Plugin name
        #[arg(value_name = "NAME")]
        name: String,

        /// Show all available versions
        #[arg(long)]
        versions: bool,
    },
    /// Cache a plugin for offline use
    Cache {
        /// Plugin name (or use --all)
        #[arg(value_name = "NAME", required_unless_present = "all")]
        name: Option<String>,

        /// Cache all plugins used in current project
        #[arg(long, conflicts_with = "name")]
        all: bool,
    },
    /// Clear plugin cache
    ClearCache {
        /// Clear specific plugin (or all if not specified)
        #[arg(value_name = "NAME")]
        name: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { path, check_ids } => {
            let tree = match cdm::FileResolver::load(&path) {
                Ok(tree) => tree,
                Err(diagnostics) => {
                    for diagnostic in &diagnostics {
                        eprintln!("{}", diagnostic);
                    }
                    std::process::exit(1);
                }
            };

            match cdm::validate_tree_with_options(tree, check_ids) {
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
                PluginCommands::List { cached } => {
                    if let Err(err) = cdm::list_plugins(cached) {
                        eprintln!("Failed to list plugins: {}", err);
                        std::process::exit(1);
                    }
                }
                PluginCommands::Info { name, versions } => {
                    if let Err(err) = cdm::plugin_info(&name, versions) {
                        eprintln!("Failed to get plugin info: {}", err);
                        std::process::exit(1);
                    }
                }
                PluginCommands::Cache { name, all } => {
                    if let Err(err) = cdm::cache_plugin_cmd(name.as_deref(), all) {
                        eprintln!("Failed to cache plugin: {}", err);
                        std::process::exit(1);
                    }
                }
                PluginCommands::ClearCache { name } => {
                    if let Err(err) = cdm::clear_cache_cmd(name.as_deref()) {
                        eprintln!("Failed to clear cache: {}", err);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Format { files, assign_ids, check, indent } => {
            // Expand glob patterns
            let mut paths = Vec::new();
            for pattern in &files {
                match glob::glob(pattern) {
                    Ok(entries) => {
                        for entry in entries {
                            match entry {
                                Ok(path) => paths.push(path),
                                Err(e) => {
                                    eprintln!("Error reading path: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Invalid glob pattern '{}': {}", pattern, e);
                        std::process::exit(1);
                    }
                }
            }

            if paths.is_empty() {
                eprintln!("No files matched the provided patterns");
                std::process::exit(1);
            }

            // Create format options
            let options = cdm::FormatOptions {
                assign_ids,
                check,
                write: !check, // Don't write if --check is set
                indent_size: indent,
                format_whitespace: true, // Always format whitespace by default
            };

            // Format files
            match cdm::format_files(&paths, &options) {
                Ok(results) => {
                    let mut total_modified = 0;
                    let mut total_assignments = 0;

                    for result in &results {
                        if result.modified {
                            total_modified += 1;

                            if assign_ids && !result.assignments.is_empty() {
                                println!("{}:", result.path.display());
                                for assignment in &result.assignments {
                                    match assignment.entity_type {
                                        cdm::EntityType::Field => {
                                            println!(
                                                "  {} '{}.{}' -> #{}",
                                                assignment.entity_type,
                                                assignment.model_name.as_ref().unwrap(),
                                                assignment.entity_name,
                                                assignment.assigned_id
                                            );
                                        }
                                        _ => {
                                            println!(
                                                "  {} '{}' -> #{}",
                                                assignment.entity_type,
                                                assignment.entity_name,
                                                assignment.assigned_id
                                            );
                                        }
                                    }
                                }
                                total_assignments += result.assignments.len();
                            }
                        }
                    }

                    if check {
                        if total_modified > 0 {
                            println!("\n{} file(s) need formatting", total_modified);
                            std::process::exit(1);
                        } else {
                            println!("All files are properly formatted");
                        }
                    } else {
                        if total_modified > 0 {
                            println!("\nFormatted {} file(s)", total_modified);
                            if assign_ids {
                                println!("Assigned {} entity ID(s)", total_assignments);
                            }
                        } else {
                            println!("No changes needed");
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
    }

    Ok(())
}