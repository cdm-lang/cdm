// src/main.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::{Context, Result};
use std::fs::read_to_string;

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { path } => {
            let source = read_to_string(&path)
                .with_context(|| format!("Failed to read file: {}", path.display()))?;

            // @todo: get appropriate ancestors
            let result = cdm::validate(&source, &[]);

            for diagnostic in &result.diagnostics {
                println!("{}", diagnostic);
            }

            if result.has_errors() {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}