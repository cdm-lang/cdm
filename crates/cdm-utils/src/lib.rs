//! Shared utilities for CDM
//!
//! This crate contains core types, parsing functions, and utilities shared between
//! the CDM compiler and related crates to avoid circular dependencies.

/// Position in source code (line and column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Span in source code (start and end positions)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

/// Parsed representation of a CDM type expression
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedType {
    /// Primitive type: string, number, boolean
    Primitive(PrimitiveType),
    /// String literal: "active", "pending"
    Literal(String),
    /// Reference to a model or type alias: User, Email
    Reference(String),
    /// Array type: User[], string[]
    Array(Box<ParsedType>),
    /// Union type: string | number, User | null
    Union(Vec<ParsedType>),
    /// Null type
    Null,
}

/// CDM primitive types
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
}

/// A fully resolved schema after applying inheritance and removals.
///
/// This represents the final state of definitions available in a file,
/// including inherited definitions from ancestors.
#[derive(Debug)]
pub struct ResolvedSchema {
    /// All available type aliases (name → resolved definition)
    pub type_aliases: std::collections::HashMap<String, ResolvedTypeAlias>,
    /// All available models (name → resolved model)
    pub models: std::collections::HashMap<String, ResolvedModel>,
}

impl ResolvedSchema {
    pub fn new() -> Self {
        Self {
            type_aliases: std::collections::HashMap::new(),
            models: std::collections::HashMap::new(),
        }
    }

    /// Check if a definition (type alias or model) exists
    pub fn contains(&self, name: &str) -> bool {
        self.type_aliases.contains_key(name) || self.models.contains_key(name)
    }
}

impl Default for ResolvedSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// A resolved type alias with source tracking
#[derive(Debug)]
pub struct ResolvedTypeAlias {
    pub name: String,
    /// The type expression as a string
    pub type_expr: String,
    /// Type identifiers referenced by this type alias
    pub references: Vec<String>,
    /// Plugin-specific configurations (plugin_name → config)
    pub plugin_configs: std::collections::HashMap<String, serde_json::Value>,
    /// Which file this definition came from (for error reporting)
    pub source_file: String,
    /// Span in the source file
    pub source_span: Span,
    /// Cached parsed type (lazy-initialized on first access)
    #[allow(clippy::type_complexity)]
    pub cached_parsed_type: std::cell::RefCell<Option<Result<ParsedType, String>>>,
}

impl Clone for ResolvedTypeAlias {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            type_expr: self.type_expr.clone(),
            references: self.references.clone(),
            plugin_configs: self.plugin_configs.clone(),
            source_file: self.source_file.clone(),
            source_span: self.source_span,
            // Don't clone the cache - let each clone re-parse if needed
            cached_parsed_type: std::cell::RefCell::new(None),
        }
    }
}

impl ResolvedTypeAlias {
    /// Create a new ResolvedTypeAlias (primarily for testing)
    pub fn new(
        name: String,
        type_expr: String,
        references: Vec<String>,
        source_file: String,
        source_span: Span,
    ) -> Self {
        Self {
            name,
            type_expr,
            references,
            plugin_configs: std::collections::HashMap::new(),
            source_file,
            source_span,
            cached_parsed_type: std::cell::RefCell::new(None),
        }
    }

    /// Get the parsed type for this type alias, parsing and caching on first access.
    pub fn parsed_type(&self) -> Result<ParsedType, String> {
        // Check cache first
        if let Some(cached) = self.cached_parsed_type.borrow().as_ref() {
            return cached.clone();
        }

        // Parse the type
        let result = parse_type_string(&self.type_expr);

        // Cache and return
        *self.cached_parsed_type.borrow_mut() = Some(result.clone());
        result
    }
}

/// A resolved model with source tracking
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    pub name: String,
    /// All fields in this model (including inherited fields)
    pub fields: Vec<ResolvedField>,
    /// Parent models this model extends from
    pub parents: Vec<String>,
    /// Plugin-specific configurations (plugin_name → config)
    pub plugin_configs: std::collections::HashMap<String, serde_json::Value>,
    /// Which file this model was defined in
    pub source_file: String,
    /// Span in the source file
    pub source_span: Span,
}

/// A resolved field with source tracking
#[derive(Debug)]
pub struct ResolvedField {
    pub name: String,
    /// The type expression as a string (None for untyped fields defaulting to string)
    pub type_expr: Option<String>,
    pub optional: bool,
    /// Default value for this field
    pub default_value: Option<serde_json::Value>,
    /// Plugin-specific configurations (plugin_name → config)
    pub plugin_configs: std::collections::HashMap<String, serde_json::Value>,
    /// Which file this field came from (original definition or inheritance)
    pub source_file: String,
    /// Span in the source file
    pub source_span: Span,
    /// Cached parsed type (lazy-initialized on first access)
    #[allow(clippy::type_complexity)]
    pub cached_parsed_type: std::cell::RefCell<Option<Result<ParsedType, String>>>,
}

impl Clone for ResolvedField {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            type_expr: self.type_expr.clone(),
            optional: self.optional,
            default_value: self.default_value.clone(),
            plugin_configs: self.plugin_configs.clone(),
            source_file: self.source_file.clone(),
            source_span: self.source_span,
            // Don't clone the cache - let each clone re-parse if needed
            cached_parsed_type: std::cell::RefCell::new(None),
        }
    }
}

impl ResolvedField {
    /// Create a new ResolvedField (primarily for testing)
    pub fn new(
        name: String,
        type_expr: Option<String>,
        optional: bool,
        source_file: String,
        source_span: Span,
    ) -> Self {
        Self {
            name,
            type_expr,
            optional,
            default_value: None,
            plugin_configs: std::collections::HashMap::new(),
            source_file,
            source_span,
            cached_parsed_type: std::cell::RefCell::new(None),
        }
    }

    /// Get the parsed type for this field, parsing and caching on first access.
    /// Returns the default type (Primitive(String)) for untyped fields.
    pub fn parsed_type(&self) -> Result<ParsedType, String> {
        // Check cache first
        if let Some(cached) = self.cached_parsed_type.borrow().as_ref() {
            return cached.clone();
        }

        // Parse the type
        let result = match &self.type_expr {
            Some(type_str) => parse_type_string(type_str),
            None => Ok(ParsedType::Primitive(PrimitiveType::String)), // Default to string
        };

        // Cache and return
        *self.cached_parsed_type.borrow_mut() = Some(result.clone());
        result
    }
}

/// Parse a CDM type string into a ParsedType
fn parse_type_string(type_str: &str) -> Result<ParsedType, String> {
    let trimmed = type_str.trim();

    // Check for union (contains | outside of quotes)
    if let Some(union_parts) = parse_union(trimmed) {
        if union_parts.len() > 1 {
            let mut parsed_parts = Vec::new();
            for part in union_parts {
                parsed_parts.push(parse_type_string(part.trim())?);
            }
            return Ok(ParsedType::Union(parsed_parts));
        }
    }

    // Check for array (ends with [])
    if trimmed.ends_with("[]") {
        let inner = &trimmed[..trimmed.len() - 2];
        let inner_type = parse_type_string(inner)?;
        return Ok(ParsedType::Array(Box::new(inner_type)));
    }

    // Check for string literal (wrapped in quotes)
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
        let literal = &trimmed[1..trimmed.len() - 1];
        return Ok(ParsedType::Literal(literal.to_string()));
    }

    // Check for primitives and special types
    match trimmed {
        "string" => Ok(ParsedType::Primitive(PrimitiveType::String)),
        "number" => Ok(ParsedType::Primitive(PrimitiveType::Number)),
        "boolean" => Ok(ParsedType::Primitive(PrimitiveType::Boolean)),
        "null" => Ok(ParsedType::Null),
        "" => Err("Empty type string".to_string()),
        _ => {
            // Must be a reference to a model or type alias
            if is_valid_identifier(trimmed) {
                Ok(ParsedType::Reference(trimmed.to_string()))
            } else {
                Err(format!("Invalid type identifier: '{}'", trimmed))
            }
        }
    }
}

/// Parse a union type, splitting on | outside of quotes
/// Returns None if no union, Some(vec) with parts if union found
fn parse_union(s: &str) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let mut current_start = 0;
    let mut in_quotes = false;
    let mut quote_char = '"';

    for (i, ch) in s.char_indices() {
        match ch {
            '"' | '\'' => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quotes = false;
                }
            }
            '|' if !in_quotes => {
                parts.push(&s[current_start..i]);
                current_start = i + 1;
            }
            _ => {}
        }
    }

    // Add the last part
    parts.push(&s[current_start..]);

    if parts.len() > 1 {
        Some(parts)
    } else {
        None
    }
}

/// Check if a string is a valid CDM identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Must start with letter or underscore
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Rest must be alphanumeric or underscore
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Find all references to a specific definition name in the resolved schema.
///
/// Returns a list of reference locations in the format:
/// - "type alias 'Name'" for type aliases that reference it
/// - "Model.field" for model fields that reference it
///
/// Includes source file information for inherited references.
pub fn find_references_in_resolved(
    resolved: &ResolvedSchema,
    target_name: &str,
) -> Vec<String> {
    let mut references = Vec::new();

    // Check type aliases that reference the target
    for (alias_name, alias) in &resolved.type_aliases {
        if alias.references.contains(&target_name.to_string()) {
            if alias.source_file == "current file" {
                references.push(format!("type alias '{}'", alias_name));
            } else {
                references.push(format!(
                    "type alias '{}' (inherited from {})",
                    alias_name, alias.source_file
                ));
            }
        }
    }

    // Check model fields that reference the target
    for (model_name, model) in &resolved.models {
        for field in &model.fields {
            if let Some(type_expr) = &field.type_expr {
                if field_type_references_definition(type_expr, target_name) {
                    if field.source_file == "current file" {
                        references.push(format!("{}.{}", model_name, field.name));
                    } else {
                        references.push(format!(
                            "{}.{} (inherited from {})",
                            model_name, field.name, field.source_file
                        ));
                    }
                }
            }
        }
    }

    references
}

/// Check if a field's type expression references a specific definition
fn field_type_references_definition(type_expr: &str, definition_name: &str) -> bool {
    // Split on non-identifier characters and check for exact match
    // This handles: TypeName, TypeName[], "literal" | TypeName, etc.
    type_expr
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| word == definition_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_primitives() {
        assert_eq!(
            parse_type_string("string"),
            Ok(ParsedType::Primitive(PrimitiveType::String))
        );
        assert_eq!(
            parse_type_string("number"),
            Ok(ParsedType::Primitive(PrimitiveType::Number))
        );
        assert_eq!(
            parse_type_string("boolean"),
            Ok(ParsedType::Primitive(PrimitiveType::Boolean))
        );
        assert_eq!(
            parse_type_string("null"),
            Ok(ParsedType::Null)
        );
    }

    #[test]
    fn test_parse_primitives_with_whitespace() {
        assert_eq!(
            parse_type_string("  string  "),
            Ok(ParsedType::Primitive(PrimitiveType::String))
        );
        assert_eq!(
            parse_type_string(" number\t"),
            Ok(ParsedType::Primitive(PrimitiveType::Number))
        );
    }

    #[test]
    fn test_parse_references() {
        assert_eq!(
            parse_type_string("User"),
            Ok(ParsedType::Reference("User".to_string()))
        );
        assert_eq!(
            parse_type_string("EmailAddress"),
            Ok(ParsedType::Reference("EmailAddress".to_string()))
        );
        assert_eq!(
            parse_type_string("_internal"),
            Ok(ParsedType::Reference("_internal".to_string()))
        );
    }

    #[test]
    fn test_parse_string_literals() {
        assert_eq!(
            parse_type_string(r#""active""#),
            Ok(ParsedType::Literal("active".to_string()))
        );
        assert_eq!(
            parse_type_string(r#""pending""#),
            Ok(ParsedType::Literal("pending".to_string()))
        );
        assert_eq!(
            parse_type_string(r#"'completed'"#),
            Ok(ParsedType::Literal("completed".to_string()))
        );
    }

    #[test]
    fn test_parse_arrays() {
        assert_eq!(
            parse_type_string("string[]"),
            Ok(ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String))))
        );
        assert_eq!(
            parse_type_string("User[]"),
            Ok(ParsedType::Array(Box::new(ParsedType::Reference("User".to_string()))))
        );
        // Nested arrays
        assert_eq!(
            parse_type_string("string[][]"),
            Ok(ParsedType::Array(Box::new(
                ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)))
            )))
        );
    }

    #[test]
    fn test_parse_unions() {
        // Simple union
        let result = parse_type_string("string | number").unwrap();
        match result {
            ParsedType::Union(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], ParsedType::Primitive(PrimitiveType::String));
                assert_eq!(parts[1], ParsedType::Primitive(PrimitiveType::Number));
            }
            _ => panic!("Expected Union type"),
        }

        // Union with null
        let result = parse_type_string("User | null").unwrap();
        match result {
            ParsedType::Union(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], ParsedType::Reference("User".to_string()));
                assert_eq!(parts[1], ParsedType::Null);
            }
            _ => panic!("Expected Union type"),
        }

        // Three-way union
        let result = parse_type_string("string | number | boolean").unwrap();
        match result {
            ParsedType::Union(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], ParsedType::Primitive(PrimitiveType::String));
                assert_eq!(parts[1], ParsedType::Primitive(PrimitiveType::Number));
                assert_eq!(parts[2], ParsedType::Primitive(PrimitiveType::Boolean));
            }
            _ => panic!("Expected Union type"),
        }
    }

    #[test]
    fn test_parse_union_with_literals() {
        let result = parse_type_string(r#""active" | "pending" | "completed""#).unwrap();
        match result {
            ParsedType::Union(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], ParsedType::Literal("active".to_string()));
                assert_eq!(parts[1], ParsedType::Literal("pending".to_string()));
                assert_eq!(parts[2], ParsedType::Literal("completed".to_string()));
            }
            _ => panic!("Expected Union type"),
        }
    }

    #[test]
    fn test_parse_complex_types() {
        // Array union
        let result = parse_type_string("string[] | number[]").unwrap();
        match result {
            ParsedType::Union(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(parts[0], ParsedType::Array(_)));
                assert!(matches!(parts[1], ParsedType::Array(_)));
            }
            _ => panic!("Expected Union type"),
        }

        // Union of references
        let result = parse_type_string("User | Admin | Guest").unwrap();
        match result {
            ParsedType::Union(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], ParsedType::Reference("User".to_string()));
                assert_eq!(parts[1], ParsedType::Reference("Admin".to_string()));
                assert_eq!(parts[2], ParsedType::Reference("Guest".to_string()));
            }
            _ => panic!("Expected Union type"),
        }
    }

    #[test]
    fn test_parse_errors() {
        // Empty string
        assert!(parse_type_string("").is_err());

        // Invalid identifier (starts with number)
        assert!(parse_type_string("9User").is_err());

        // Invalid identifier (special characters)
        assert!(parse_type_string("User-Name").is_err());
    }

    #[test]
    fn test_is_valid_identifier() {
        // Valid identifiers
        assert!(is_valid_identifier("User"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("User123"));
        assert!(is_valid_identifier("snake_case"));
        assert!(is_valid_identifier("PascalCase"));
        assert!(is_valid_identifier("camelCase"));

        // Invalid identifiers
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123abc"));
        assert!(!is_valid_identifier("user-name"));
        assert!(!is_valid_identifier("user.name"));
        assert!(!is_valid_identifier("user name"));
    }

    #[test]
    fn test_resolved_field_parsed_type_caching() {
        let field = ResolvedField {
            name: "test".to_string(),
            type_expr: Some("string | number".to_string()),
            optional: false,
            default_value: None,
            plugin_configs: std::collections::HashMap::new(),
            source_file: "test.cdm".to_string(),
            source_span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 10 },
            },
            cached_parsed_type: std::cell::RefCell::new(None),
        };

        // First call should parse
        let result1 = field.parsed_type().unwrap();
        assert!(matches!(result1, ParsedType::Union(_)));

        // Second call should return cached result
        let result2 = field.parsed_type().unwrap();
        assert_eq!(result1, result2);

        // Verify cache is populated
        assert!(field.cached_parsed_type.borrow().is_some());
    }

    #[test]
    fn test_resolved_field_default_type() {
        let field = ResolvedField {
            name: "test".to_string(),
            type_expr: None, // No type specified
            optional: false,
            default_value: None,
            plugin_configs: std::collections::HashMap::new(),
            source_file: "test.cdm".to_string(),
            source_span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 10 },
            },
            cached_parsed_type: std::cell::RefCell::new(None),
        };

        // Should default to string
        let result = field.parsed_type().unwrap();
        assert_eq!(result, ParsedType::Primitive(PrimitiveType::String));
    }

    #[test]
    fn test_resolved_type_alias_parsed_type() {
        let alias = ResolvedTypeAlias {
            name: "Status".to_string(),
            type_expr: r#""active" | "pending""#.to_string(),
            references: vec![],
            plugin_configs: std::collections::HashMap::new(),
            source_file: "test.cdm".to_string(),
            source_span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 10 },
            },
            cached_parsed_type: std::cell::RefCell::new(None),
        };

        let result = alias.parsed_type().unwrap();
        match result {
            ParsedType::Union(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], ParsedType::Literal("active".to_string()));
                assert_eq!(parts[1], ParsedType::Literal("pending".to_string()));
            }
            _ => panic!("Expected Union type"),
        }
    }

    #[test]
    fn test_resolved_field_with_default_value() {
        let mut field = ResolvedField::new(
            "name".to_string(),
            Some("string".to_string()),
            false,
            "test.cdm".to_string(),
            Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 10 },
            },
        );

        // Initially no default value
        assert!(field.default_value.is_none());

        // Set a default value
        field.default_value = Some(serde_json::json!("John Doe"));
        assert!(field.default_value.is_some());
        assert_eq!(field.default_value.unwrap(), serde_json::json!("John Doe"));
    }

    #[test]
    fn test_resolved_field_with_plugin_configs() {
        let mut field = ResolvedField::new(
            "email".to_string(),
            Some("string".to_string()),
            false,
            "test.cdm".to_string(),
            Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 10 },
            },
        );

        // Initially no plugin configs
        assert!(field.plugin_configs.is_empty());

        // Add plugin configs
        field.plugin_configs.insert(
            "validator".to_string(),
            serde_json::json!({"format": "email"}),
        );
        field.plugin_configs.insert(
            "docs".to_string(),
            serde_json::json!({"description": "User email address"}),
        );

        assert_eq!(field.plugin_configs.len(), 2);
        assert_eq!(
            field.plugin_configs.get("validator").unwrap(),
            &serde_json::json!({"format": "email"})
        );
        assert_eq!(
            field.plugin_configs.get("docs").unwrap(),
            &serde_json::json!({"description": "User email address"})
        );
    }

    #[test]
    fn test_resolved_model_with_parents() {
        let span = Span {
            start: Position { line: 0, column: 0 },
            end: Position { line: 0, column: 10 },
        };

        let model = ResolvedModel {
            name: "AdminUser".to_string(),
            fields: vec![],
            parents: vec!["User".to_string(), "Timestamped".to_string()],
            plugin_configs: std::collections::HashMap::new(),
            source_file: "test.cdm".to_string(),
            source_span: span,
        };

        assert_eq!(model.parents.len(), 2);
        assert_eq!(model.parents[0], "User");
        assert_eq!(model.parents[1], "Timestamped");
    }

    #[test]
    fn test_resolved_model_with_plugin_configs() {
        let span = Span {
            start: Position { line: 0, column: 0 },
            end: Position { line: 0, column: 10 },
        };

        let mut plugin_configs = std::collections::HashMap::new();
        plugin_configs.insert(
            "prisma".to_string(),
            serde_json::json!({"tableName": "users"}),
        );
        plugin_configs.insert(
            "docs".to_string(),
            serde_json::json!({"description": "User model"}),
        );

        let model = ResolvedModel {
            name: "User".to_string(),
            fields: vec![],
            parents: vec![],
            plugin_configs,
            source_file: "test.cdm".to_string(),
            source_span: span,
        };

        assert_eq!(model.plugin_configs.len(), 2);
        assert_eq!(
            model.plugin_configs.get("prisma").unwrap(),
            &serde_json::json!({"tableName": "users"})
        );
    }

    #[test]
    fn test_resolved_type_alias_with_plugin_configs() {
        let mut plugin_configs = std::collections::HashMap::new();
        plugin_configs.insert(
            "docs".to_string(),
            serde_json::json!({"description": "User status type"}),
        );

        let alias = ResolvedTypeAlias {
            name: "Status".to_string(),
            type_expr: r#""active" | "pending""#.to_string(),
            references: vec![],
            plugin_configs,
            source_file: "test.cdm".to_string(),
            source_span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 10 },
            },
            cached_parsed_type: std::cell::RefCell::new(None),
        };

        assert_eq!(alias.plugin_configs.len(), 1);
        assert_eq!(
            alias.plugin_configs.get("docs").unwrap(),
            &serde_json::json!({"description": "User status type"})
        );
    }

    #[test]
    fn test_resolved_field_clone_preserves_new_fields() {
        let mut original = ResolvedField::new(
            "name".to_string(),
            Some("string".to_string()),
            false,
            "test.cdm".to_string(),
            Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 10 },
            },
        );

        original.default_value = Some(serde_json::json!("default"));
        original.plugin_configs.insert(
            "test".to_string(),
            serde_json::json!({"key": "value"}),
        );

        let cloned = original.clone();

        assert_eq!(cloned.default_value, original.default_value);
        assert_eq!(cloned.plugin_configs.len(), 1);
        assert_eq!(
            cloned.plugin_configs.get("test").unwrap(),
            &serde_json::json!({"key": "value"})
        );
    }

    #[test]
    fn test_resolved_model_clone_preserves_new_fields() {
        let span = Span {
            start: Position { line: 0, column: 0 },
            end: Position { line: 0, column: 10 },
        };

        let mut plugin_configs = std::collections::HashMap::new();
        plugin_configs.insert(
            "test".to_string(),
            serde_json::json!({"key": "value"}),
        );

        let original = ResolvedModel {
            name: "User".to_string(),
            fields: vec![],
            parents: vec!["Base".to_string()],
            plugin_configs,
            source_file: "test.cdm".to_string(),
            source_span: span,
        };

        let cloned = original.clone();

        assert_eq!(cloned.parents, original.parents);
        assert_eq!(cloned.plugin_configs.len(), 1);
        assert_eq!(
            cloned.plugin_configs.get("test").unwrap(),
            &serde_json::json!({"key": "value"})
        );
    }

    #[test]
    fn test_resolved_type_alias_clone_preserves_new_fields() {
        let mut plugin_configs = std::collections::HashMap::new();
        plugin_configs.insert(
            "test".to_string(),
            serde_json::json!({"key": "value"}),
        );

        let original = ResolvedTypeAlias {
            name: "Status".to_string(),
            type_expr: "string".to_string(),
            references: vec![],
            plugin_configs,
            source_file: "test.cdm".to_string(),
            source_span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 10 },
            },
            cached_parsed_type: std::cell::RefCell::new(None),
        };

        let cloned = original.clone();

        assert_eq!(cloned.plugin_configs.len(), 1);
        assert_eq!(
            cloned.plugin_configs.get("test").unwrap(),
            &serde_json::json!({"key": "value"})
        );
    }
}
