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
    /// Validate a CDM file and print its AST
    Validate {
        /// Path to the .cdm file to validate
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

            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&grammar::LANGUAGE.into())?;

            println!("validate! {} {}", path.display(), source);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}