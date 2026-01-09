use cdm_plugin_interface::{ConfigLevel, PathSegment, Severity, Utils, ValidationError, JSON};

/// Validates plugin configuration at different levels
pub fn validate_config(
    level: ConfigLevel,
    config: JSON,
    _utils: &Utils,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    match level {
        ConfigLevel::Global => {
            validate_global_config(&config, &mut errors);
        }
        ConfigLevel::TypeAlias { ref name } => {
            validate_type_alias_config(&config, name, &mut errors);
        }
        ConfigLevel::Model { ref name } => {
            validate_model_config(&config, name, &mut errors);
        }
        ConfigLevel::Field {
            ref model,
            ref field,
        } => {
            validate_field_config(&config, model, field, &mut errors);
        }
    }

    errors
}

fn validate_global_config(config: &JSON, errors: &mut Vec<ValidationError>) {
    // Validate entity_file_strategy
    if let Some(strategy) = config.get("entity_file_strategy") {
        if let Some(strategy_str) = strategy.as_str() {
            if !["single", "per_model"].contains(&strategy_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "entity_file_strategy".to_string(),
                    }],
                    message: "entity_file_strategy must be 'single' or 'per_model'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate table_name_format
    if let Some(format) = config.get("table_name_format") {
        if let Some(format_str) = format.as_str() {
            if !["snake_case", "preserve", "camel_case", "pascal_case"].contains(&format_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "table_name_format".to_string(),
                    }],
                    message: "table_name_format must be 'snake_case', 'preserve', 'camel_case', or 'pascal_case'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate column_name_format
    if let Some(format) = config.get("column_name_format") {
        if let Some(format_str) = format.as_str() {
            if !["snake_case", "preserve", "camel_case", "pascal_case"].contains(&format_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "column_name_format".to_string(),
                    }],
                    message: "column_name_format must be 'snake_case', 'preserve', 'camel_case', or 'pascal_case'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate typeorm_import_path is non-empty if specified
    if let Some(path) = config.get("typeorm_import_path") {
        if let Some(path_str) = path.as_str() {
            if path_str.is_empty() {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "typeorm_import_path".to_string(),
                    }],
                    message: "typeorm_import_path cannot be empty".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }
}

fn validate_type_alias_config(
    config: &JSON,
    alias_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate column_type is a valid string if present
    if let Some(col_type) = config.get("column_type") {
        if !col_type.is_string() {
            errors.push(ValidationError {
                path: vec![
                    PathSegment {
                        kind: "type_alias".to_string(),
                        name: alias_name.to_string(),
                    },
                    PathSegment {
                        kind: "config".to_string(),
                        name: "column_type".to_string(),
                    },
                ],
                message: "column_type must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_model_config(config: &JSON, model_name: &str, errors: &mut Vec<ValidationError>) {
    // Validate indexes
    if let Some(indexes) = config.get("indexes") {
        if let Some(indexes_array) = indexes.as_array() {
            for (i, index) in indexes_array.iter().enumerate() {
                // Validate fields is non-empty array
                if let Some(fields) = index.get("fields") {
                    if let Some(fields_array) = fields.as_array() {
                        if fields_array.is_empty() {
                            errors.push(ValidationError {
                                path: vec![
                                    PathSegment {
                                        kind: "model".to_string(),
                                        name: model_name.to_string(),
                                    },
                                    PathSegment {
                                        kind: "config".to_string(),
                                        name: format!("indexes[{}]", i),
                                    },
                                ],
                                message: "index must have at least one field".to_string(),
                                severity: Severity::Error,
                            });
                        }
                    }
                } else {
                    errors.push(ValidationError {
                        path: vec![
                            PathSegment {
                                kind: "model".to_string(),
                                name: model_name.to_string(),
                            },
                            PathSegment {
                                kind: "config".to_string(),
                                name: format!("indexes[{}]", i),
                            },
                        ],
                        message: "index must have a 'fields' array".to_string(),
                        severity: Severity::Error,
                    });
                }
            }
        }
    }

    // Note: We don't warn about missing primary keys here since that validation
    // requires schema context (to check all fields). The user explicitly chose
    // "require explicit configuration" so we trust they will configure PKs.
}

fn validate_field_config(
    config: &JSON,
    model_name: &str,
    field_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate primary key config
    if let Some(primary) = config.get("primary") {
        if let Some(generation) = primary.get("generation") {
            if let Some(gen_str) = generation.as_str() {
                if !["increment", "uuid", "identity", "rowid"].contains(&gen_str) {
                    errors.push(ValidationError {
                        path: vec![
                            PathSegment {
                                kind: "model".to_string(),
                                name: model_name.to_string(),
                            },
                            PathSegment {
                                kind: "field".to_string(),
                                name: field_name.to_string(),
                            },
                            PathSegment {
                                kind: "config".to_string(),
                                name: "primary.generation".to_string(),
                            },
                        ],
                        message: "primary.generation must be 'increment', 'uuid', 'identity', or 'rowid'".to_string(),
                        severity: Severity::Error,
                    });
                }
            }
        }
    }

    // Validate relation config
    if let Some(relation) = config.get("relation") {
        validate_relation(relation, model_name, field_name, errors);
    }

    // Error if both primary and relation are specified
    if config.get("primary").is_some() && config.get("relation").is_some() {
        errors.push(ValidationError {
            path: vec![
                PathSegment {
                    kind: "model".to_string(),
                    name: model_name.to_string(),
                },
                PathSegment {
                    kind: "field".to_string(),
                    name: field_name.to_string(),
                },
            ],
            message: "field cannot have both 'primary' and 'relation' configuration".to_string(),
            severity: Severity::Error,
        });
    }
}

fn validate_relation(
    relation: &JSON,
    model_name: &str,
    field_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate relation type is required
    if let Some(rel_type) = relation.get("type") {
        if let Some(type_str) = rel_type.as_str() {
            if !["one_to_one", "one_to_many", "many_to_one", "many_to_many"].contains(&type_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "field".to_string(),
                            name: field_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "relation.type".to_string(),
                        },
                    ],
                    message: "relation.type must be 'one_to_one', 'one_to_many', 'many_to_one', or 'many_to_many'".to_string(),
                    severity: Severity::Error,
                });
            }

            // Validate many_to_many requires join_table
            if type_str == "many_to_many" {
                if let Some(join_table) = relation.get("join_table") {
                    // Validate join_table.name is required and non-empty
                    if let Some(name) = join_table.get("name") {
                        if let Some(name_str) = name.as_str() {
                            if name_str.is_empty() {
                                errors.push(ValidationError {
                                    path: vec![
                                        PathSegment {
                                            kind: "model".to_string(),
                                            name: model_name.to_string(),
                                        },
                                        PathSegment {
                                            kind: "field".to_string(),
                                            name: field_name.to_string(),
                                        },
                                        PathSegment {
                                            kind: "config".to_string(),
                                            name: "relation.join_table.name".to_string(),
                                        },
                                    ],
                                    message: "join_table name cannot be empty".to_string(),
                                    severity: Severity::Error,
                                });
                            }
                        }
                    } else {
                        errors.push(ValidationError {
                            path: vec![
                                PathSegment {
                                    kind: "model".to_string(),
                                    name: model_name.to_string(),
                                },
                                PathSegment {
                                    kind: "field".to_string(),
                                    name: field_name.to_string(),
                                },
                                PathSegment {
                                    kind: "config".to_string(),
                                    name: "relation.join_table".to_string(),
                                },
                            ],
                            message: "join_table must have a 'name' field".to_string(),
                            severity: Severity::Error,
                        });
                    }
                }
                // Note: join_table is optional for ManyToMany, TypeORM can auto-generate
            }
        }
    } else {
        errors.push(ValidationError {
            path: vec![
                PathSegment {
                    kind: "model".to_string(),
                    name: model_name.to_string(),
                },
                PathSegment {
                    kind: "field".to_string(),
                    name: field_name.to_string(),
                },
                PathSegment {
                    kind: "config".to_string(),
                    name: "relation".to_string(),
                },
            ],
            message: "relation must have a 'type' field".to_string(),
            severity: Severity::Error,
        });
    }

    // Validate inverse_side is a valid identifier if specified
    if let Some(inverse) = relation.get("inverse_side") {
        if let Some(inverse_str) = inverse.as_str() {
            if inverse_str.is_empty() {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "field".to_string(),
                            name: field_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "relation.inverse_side".to_string(),
                        },
                    ],
                    message: "inverse_side cannot be empty".to_string(),
                    severity: Severity::Error,
                });
            } else if !is_valid_identifier(inverse_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "field".to_string(),
                            name: field_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "relation.inverse_side".to_string(),
                        },
                    ],
                    message: "inverse_side must be a valid JavaScript identifier".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate on_delete action
    if let Some(on_delete) = relation.get("on_delete") {
        if let Some(action) = on_delete.as_str() {
            if !["CASCADE", "SET NULL", "RESTRICT", "NO ACTION", "DEFAULT"].contains(&action) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "field".to_string(),
                            name: field_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "relation.on_delete".to_string(),
                        },
                    ],
                    message: "on_delete must be 'CASCADE', 'SET NULL', 'RESTRICT', 'NO ACTION', or 'DEFAULT'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate on_update action
    if let Some(on_update) = relation.get("on_update") {
        if let Some(action) = on_update.as_str() {
            if !["CASCADE", "SET NULL", "RESTRICT", "NO ACTION", "DEFAULT"].contains(&action) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "field".to_string(),
                            name: field_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "relation.on_update".to_string(),
                        },
                    ],
                    message: "on_update must be 'CASCADE', 'SET NULL', 'RESTRICT', 'NO ACTION', or 'DEFAULT'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }
}

/// Check if a string is a valid JavaScript/TypeScript identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();

    // First character must be letter, underscore, or dollar sign
    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' && first != '$' {
        return false;
    }

    // Remaining characters must be alphanumeric, underscore, or dollar sign
    for c in chars {
        if !c.is_alphanumeric() && c != '_' && c != '$' {
            return false;
        }
    }

    // Check against reserved words
    const RESERVED_WORDS: &[&str] = &[
        "break", "case", "catch", "continue", "debugger", "default", "delete",
        "do", "else", "finally", "for", "function", "if", "in", "instanceof",
        "new", "return", "switch", "this", "throw", "try", "typeof", "var",
        "void", "while", "with", "class", "const", "enum", "export", "extends",
        "import", "super", "implements", "interface", "let", "package", "private",
        "protected", "public", "static", "yield", "null", "true", "false",
    ];

    !RESERVED_WORDS.contains(&s)
}

#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;
