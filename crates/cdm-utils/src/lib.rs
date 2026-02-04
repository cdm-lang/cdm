//! Shared utilities for CDM
//!
//! This crate contains core types, parsing functions, and utilities shared between
//! the CDM compiler and related crates to avoid circular dependencies.

use serde::{Deserialize, Serialize};

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

/// Source of an entity ID - identifies who assigned the ID.
///
/// Entity IDs are scoped by their source to prevent collisions when multiple
/// templates use the same numeric IDs. Two entity IDs only collide if they
/// have the same source AND the same local_id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntityIdSource {
    /// Defined in the current schema being compiled (including extends file inheritance).
    /// Files without a cdm-template.json manifest are NOT templates and use this source.
    Local,
    /// From a registry template (registry enforces name uniqueness)
    Registry { name: String },
    /// From a git template
    Git {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    /// From a local filesystem template (has cdm-template.json).
    /// Path is relative to project root.
    LocalTemplate { path: String },
}

impl std::fmt::Display for EntityIdSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityIdSource::Local => write!(f, "local"),
            EntityIdSource::Registry { name } => write!(f, "{}", name),
            EntityIdSource::Git { url, path } => match path {
                Some(p) => write!(f, "git:{}#{}", url, p),
                None => write!(f, "git:{}", url),
            },
            EntityIdSource::LocalTemplate { path } => write!(f, "{}", path),
        }
    }
}

/// Composite entity ID with source tracking.
///
/// Entity IDs are used to track schema elements across versions for migration
/// purposes. The composite structure prevents collisions when multiple templates
/// use the same numeric IDs independently.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId {
    /// The source where this ID was defined
    #[serde(flatten)]
    pub source: EntityIdSource,
    /// The numeric ID value within the source
    pub local_id: u64,
}

impl EntityId {
    /// Create a new local entity ID
    pub fn local(id: u64) -> Self {
        Self {
            source: EntityIdSource::Local,
            local_id: id,
        }
    }

    /// Create a new entity ID from a registry template
    pub fn registry(name: impl Into<String>, id: u64) -> Self {
        Self {
            source: EntityIdSource::Registry { name: name.into() },
            local_id: id,
        }
    }

    /// Create a new entity ID from a git template
    pub fn git(url: impl Into<String>, path: Option<String>, id: u64) -> Self {
        Self {
            source: EntityIdSource::Git {
                url: url.into(),
                path,
            },
            local_id: id,
        }
    }

    /// Create a new entity ID from a local template
    pub fn local_template(path: impl Into<String>, id: u64) -> Self {
        Self {
            source: EntityIdSource::LocalTemplate { path: path.into() },
            local_id: id,
        }
    }

    /// Get a display string for this entity ID.
    /// For local IDs, just shows #N. For template IDs, shows source:#N.
    pub fn display(&self) -> String {
        match &self.source {
            EntityIdSource::Local => format!("#{}", self.local_id),
            EntityIdSource::Registry { name } => format!("{}:#{}", name, self.local_id),
            EntityIdSource::Git { url, path } => match path {
                Some(p) => format!("git:{}#{}:#{}", url, p, self.local_id),
                None => format!("git:{}:#{}", url, self.local_id),
            },
            EntityIdSource::LocalTemplate { path } => format!("{}:#{}", path, self.local_id),
        }
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Parsed representation of a CDM type expression
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedType {
    /// Primitive type: string, number, boolean
    Primitive(PrimitiveType),
    /// String literal: "active", "pending"
    Literal(String),
    /// Number literal: 1, 2, 3 (used in number literal unions for map keys)
    NumberLiteral(f64),
    /// Reference to a model or type alias: User, Email
    Reference(String),
    /// Array type: User[], string[]
    Array(Box<ParsedType>),
    /// Map type: User[string], Prize[1 | 2 | 3]
    Map {
        value_type: Box<ParsedType>,
        key_type: Box<ParsedType>,
    },
    /// Union type: string | number, User | null
    Union(Vec<ParsedType>),
    /// Null type
    Null,
    /// Model reference - only accepts model names (not type aliases)
    /// Used in plugin schemas to validate that a value refers to a CDM model
    ModelRef,
    /// Type reference - only accepts type alias names (not models)
    /// Used in plugin schemas to validate that a value refers to a CDM type alias
    TypeRef,
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
    /// This excludes models that are marked for removal.
    pub models: std::collections::HashMap<String, ResolvedModel>,
    /// All models including removed ones, used for config inheritance.
    /// When a child model extends a removed parent, the parent's config
    /// should still be inherited. This map includes ALL models for that purpose.
    pub all_models_for_inheritance: std::collections::HashMap<String, ResolvedModel>,
}

impl ResolvedSchema {
    pub fn new() -> Self {
        Self {
            type_aliases: std::collections::HashMap::new(),
            models: std::collections::HashMap::new(),
            all_models_for_inheritance: std::collections::HashMap::new(),
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
    /// Optional composite entity ID for migration tracking
    pub entity_id: Option<EntityId>,
    /// Whether this type alias comes from an imported template.
    /// Template type aliases are used for field resolution but should not
    /// be passed to plugins directly.
    pub is_from_template: bool,
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
            entity_id: self.entity_id.clone(),
            is_from_template: self.is_from_template,
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
            entity_id: None,
            is_from_template: false,
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
    /// Optional composite entity ID for migration tracking
    pub entity_id: Option<EntityId>,
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
    /// Optional composite entity ID for migration tracking
    pub entity_id: Option<EntityId>,
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
            entity_id: self.entity_id.clone(),
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
            entity_id: None,
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
pub fn parse_type_string(type_str: &str) -> Result<ParsedType, String> {
    let trimmed = type_str.trim();

    // Check for union (contains | outside of quotes and brackets)
    if let Some(union_parts) = parse_union(trimmed) {
        if union_parts.len() > 1 {
            let mut parsed_parts = Vec::new();
            for part in union_parts {
                parsed_parts.push(parse_type_string(part.trim())?);
            }
            return Ok(ParsedType::Union(parsed_parts));
        }
    }

    // Check for map type: ValueType[KeyType] (non-empty brackets)
    // Must check BEFORE array to distinguish Type[] from Type[Key]
    if let Some((value_part, key_part)) = parse_map_brackets(trimmed) {
        let value_type = parse_type_string(value_part)?;
        let key_type = parse_type_string(key_part)?;
        return Ok(ParsedType::Map {
            value_type: Box::new(value_type),
            key_type: Box::new(key_type),
        });
    }

    // Check for array (ends with [] - empty brackets)
    if trimmed.ends_with("[]") {
        let inner = &trimmed[..trimmed.len() - 2];
        let inner_type = parse_type_string(inner)?;
        return Ok(ParsedType::Array(Box::new(inner_type)));
    }

    // Check for string literal (wrapped in quotes)
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let literal = &trimmed[1..trimmed.len() - 1];
        return Ok(ParsedType::Literal(literal.to_string()));
    }

    // Check for number literal
    if let Ok(num) = trimmed.parse::<f64>() {
        return Ok(ParsedType::NumberLiteral(num));
    }

    // Check for primitives and special types
    match trimmed {
        "string" => Ok(ParsedType::Primitive(PrimitiveType::String)),
        "number" => Ok(ParsedType::Primitive(PrimitiveType::Number)),
        "boolean" => Ok(ParsedType::Primitive(PrimitiveType::Boolean)),
        "null" => Ok(ParsedType::Null),
        "Model" => Ok(ParsedType::ModelRef),
        "Type" => Ok(ParsedType::TypeRef),
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

/// Parse map brackets: returns (value_type, key_type) if valid map syntax
/// Handles nested maps like string[string][Locale]
fn parse_map_brackets(s: &str) -> Option<(&str, &str)> {
    // Must end with ]
    if !s.ends_with(']') {
        return None;
    }

    // Find the matching open bracket for the last close bracket
    let mut depth = 0;
    let mut last_open = None;

    for (i, ch) in s.char_indices().rev() {
        match ch {
            ']' => depth += 1,
            '[' => {
                depth -= 1;
                if depth == 0 {
                    last_open = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    let open_idx = last_open?;
    let key_content = &s[open_idx + 1..s.len() - 1];

    // Empty brackets = array, not map
    if key_content.trim().is_empty() {
        return None;
    }

    let value_part = &s[..open_idx];
    Some((value_part.trim(), key_content.trim()))
}

/// Parse a union type, splitting on | outside of quotes and brackets
/// Returns None if no union, Some(vec) with parts if union found
fn parse_union(s: &str) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let mut current_start = 0;
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut bracket_depth = 0;

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
            '[' if !in_quotes => bracket_depth += 1,
            ']' if !in_quotes => bracket_depth -= 1,
            '|' if !in_quotes && bracket_depth == 0 => {
                parts.push(&s[current_start..i]);
                current_start = i + 1;
            }
            _ => {}
        }
    }

    // Add the last part
    parts.push(&s[current_start..]);

    if parts.len() > 1 { Some(parts) } else { None }
}

/// Check if a string is a valid CDM identifier (including qualified identifiers like sql.UUID)
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Split on dots for qualified identifiers (e.g., "sql.UUID", "auth.types.Email")
    for part in s.split('.') {
        if part.is_empty() {
            return false; // No empty parts allowed
        }

        // Must start with letter or underscore
        let mut chars = part.chars();
        let first = chars.next().unwrap();
        if !first.is_alphabetic() && first != '_' {
            return false;
        }

        // Rest must be alphanumeric or underscore
        if !chars.all(|c| c.is_alphanumeric() || c == '_') {
            return false;
        }
    }

    true
}

/// Find all references to a specific definition name in the resolved schema.
///
/// Returns a list of reference locations in the format:
/// - "type alias 'Name'" for type aliases that reference it
/// - "Model.field" for model fields that reference it
///
/// Includes source file information for inherited references.
pub fn find_references_in_resolved(resolved: &ResolvedSchema, target_name: &str) -> Vec<String> {
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
#[path = "lib/lib_tests.rs"]
mod lib_tests;
