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
    }

    Ok(())
}