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
            // Load CDM code to parse from specified path
            let source: String = read_to_string(&path)
                .with_context(|| format!("Failed to read file: {}", path.display()))?;

            // Create the parser using the built language from the grammar crate
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&grammar::LANGUAGE.into())?;

            // Parse the CDM code
            let tree = parser.parse(&source, None)
                .context("Failed to parse file")?;

            print_errors(tree.root_node(), &source);
            println!("{}", tree.root_node().to_sexp());
        }
    }

    Ok(())
}

fn print_errors(node: tree_sitter::Node, source: &str) {
    if node.is_error() || node.is_missing() {
        let start = node.start_position();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("<invalid>");
        println!(
            "Error at line {}, column {}: {:?}",
            start.row + 1,
            start.column,
            text
        );
    }

    for child in node.children(&mut node.walk()) {
        print_errors(child, source);
    }
}


#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}