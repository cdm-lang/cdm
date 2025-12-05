use std::fmt;

/// A position in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,   // 0-indexed (LSP standard)
    pub column: usize, // 0-indexed
}

/// A span in source code (start inclusive, end exclusive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
    pub span: Span,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        // Display as 1-indexed for human readability
        write!(
            f,
            "{}[{}:{}]: {}",
            severity,
            self.span.start.line + 1,
            self.span.start.column + 1,
            self.message
        )
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    // Include the tree for callers that need it (e.g., code generation)
    pub tree: Option<tree_sitter::Tree>,
}

impl ValidationResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
}

/// Validate CDM source code and return all diagnostics.
/// 
/// This performs both syntactic (parse errors) and semantic 
/// (undefined types, etc.) validation.
pub fn validate(source: &str) -> ValidationResult {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // Parse
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");

    let tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => {
            diagnostics.push(Diagnostic {
                message: "Failed to parse file".to_string(),
                severity: Severity::Error,
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
            });
            return ValidationResult { diagnostics, tree: None };
        }
    };

    // Collect syntax errors from tree-sitter
    collect_syntax_errors(tree.root_node(), source, &mut diagnostics);

    // Semantic validation
    collect_semantic_errors(tree.root_node(), source, &mut diagnostics);

    ValidationResult {
        diagnostics,
        tree: Some(tree),
    }
}

fn collect_syntax_errors(
    node: tree_sitter::Node,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if node.is_error() || node.is_missing() {
        let start = node.start_position();
        let end = node.end_position();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("<invalid>");
        
        diagnostics.push(Diagnostic {
            message: format!("Syntax error: unexpected '{}'", text),
            severity: Severity::Error,
            span: Span {
                start: Position { line: start.row, column: start.column },
                end: Position { line: end.row, column: end.column },
            },
        });
    }

    for child in node.children(&mut node.walk()) {
        collect_syntax_errors(child, source, diagnostics);
    }
}

fn collect_semantic_errors(
    _root: tree_sitter::Node,
    _source: &str,
    _diagnostics: &mut Vec<Diagnostic>,
) {
    // TODO: implement type checking, undefined references, etc.
    // Two-pass: collect definitions, then check references
}