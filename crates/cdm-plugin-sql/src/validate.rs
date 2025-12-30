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
    // Validate dialect
    if let Some(dialect) = config.get("dialect") {
        if let Some(dialect_str) = dialect.as_str() {
            if !["postgresql", "sqlite"].contains(&dialect_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "dialect".to_string(),
                    }],
                    message: "dialect must be 'postgresql' or 'sqlite'".to_string(),
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

    // Validate default_string_length is positive
    if let Some(length) = config.get("default_string_length") {
        if let Some(length_num) = length.as_i64() {
            if length_num <= 0 {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "default_string_length".to_string(),
                    }],
                    message: "default_string_length must be greater than 0".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate number_type
    if let Some(num_type) = config.get("number_type") {
        if let Some(num_type_str) = num_type.as_str() {
            if !["real", "double", "numeric"].contains(&num_type_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "number_type".to_string(),
                    }],
                    message: "number_type must be 'real', 'double', or 'numeric'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate schema is only used with PostgreSQL
    if let Some(schema) = config.get("schema") {
        if schema.as_str().is_some() {
            if let Some(dialect) = config.get("dialect") {
                if let Some(dialect_str) = dialect.as_str() {
                    if dialect_str == "sqlite" {
                        errors.push(ValidationError {
                            path: vec![PathSegment {
                                kind: "global".to_string(),
                                name: "schema".to_string(),
                            }],
                            message: "schema setting is only supported for PostgreSQL dialect".to_string(),
                            severity: Severity::Error,
                        });
                    }
                }
            }
        }
    }
}

fn validate_type_alias_config(
    config: &JSON,
    alias_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Type alias configs for SQL plugin typically just define custom SQL types
    // No specific validation needed beyond schema validation, but we could add
    // checks for valid SQL type syntax in the future if needed

    // For now, we just ensure comment is a string if present
    if let Some(comment) = config.get("comment") {
        if !comment.is_string() {
            errors.push(ValidationError {
                path: vec![
                    PathSegment {
                        kind: "type_alias".to_string(),
                        name: alias_name.to_string(),
                    },
                    PathSegment {
                        kind: "config".to_string(),
                        name: "comment".to_string(),
                    },
                ],
                message: "comment must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_model_config(config: &JSON, model_name: &str, errors: &mut Vec<ValidationError>) {
    // Validate indexes
    if let Some(indexes) = config.get("indexes") {
        if let Some(indexes_array) = indexes.as_array() {
            let mut primary_key_count = 0;

            for (i, index) in indexes_array.iter().enumerate() {
                // Check if this is a primary key
                if let Some(primary) = index.get("primary") {
                    if primary.as_bool() == Some(true) {
                        primary_key_count += 1;
                    }
                }

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

                // Validate index method is PostgreSQL-only
                if let Some(method) = index.get("method") {
                    if method.as_str().is_some() {
                        // This validation needs global config context
                        // We'll add a warning suggesting to check dialect compatibility
                        errors.push(ValidationError {
                            path: vec![
                                PathSegment {
                                    kind: "model".to_string(),
                                    name: model_name.to_string(),
                                },
                                PathSegment {
                                    kind: "config".to_string(),
                                    name: format!("indexes[{}].method", i),
                                },
                            ],
                            message: "index method is only supported for PostgreSQL dialect".to_string(),
                            severity: Severity::Warning,
                        });
                    }
                }

                // Validate where clause is PostgreSQL-only
                if let Some(where_clause) = index.get("where") {
                    if where_clause.as_str().is_some() {
                        errors.push(ValidationError {
                            path: vec![
                                PathSegment {
                                    kind: "model".to_string(),
                                    name: model_name.to_string(),
                                },
                                PathSegment {
                                    kind: "config".to_string(),
                                    name: format!("indexes[{}].where", i),
                                },
                            ],
                            message: "partial index (where clause) is only supported for PostgreSQL dialect".to_string(),
                            severity: Severity::Warning,
                        });
                    }
                }
            }

            // Check for multiple primary keys
            if primary_key_count > 1 {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "indexes".to_string(),
                        },
                    ],
                    message: "a table can only have one primary key".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate constraints
    if let Some(constraints) = config.get("constraints") {
        if let Some(constraints_array) = constraints.as_array() {
            for (i, constraint) in constraints_array.iter().enumerate() {
                let constraint_type = constraint.get("type").and_then(|t| t.as_str());

                // Validate fields array exists and is non-empty
                if let Some(fields) = constraint.get("fields") {
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
                                        name: format!("constraints[{}]", i),
                                    },
                                ],
                                message: "constraint must have at least one field".to_string(),
                                severity: Severity::Error,
                            });
                        }
                    }
                }

                // Validate constraint type-specific requirements
                match constraint_type {
                    Some("default") | Some("check") | Some("custom") => {
                        if constraint.get("expression").is_none() {
                            errors.push(ValidationError {
                                path: vec![
                                    PathSegment {
                                        kind: "model".to_string(),
                                        name: model_name.to_string(),
                                    },
                                    PathSegment {
                                        kind: "config".to_string(),
                                        name: format!("constraints[{}]", i),
                                    },
                                ],
                                message: format!(
                                    "'{}' constraint requires an 'expression' field",
                                    constraint_type.unwrap()
                                ),
                                severity: Severity::Error,
                            });
                        }
                    }
                    Some("exclude") => {
                        if constraint.get("expression").is_none() {
                            errors.push(ValidationError {
                                path: vec![
                                    PathSegment {
                                        kind: "model".to_string(),
                                        name: model_name.to_string(),
                                    },
                                    PathSegment {
                                        kind: "config".to_string(),
                                        name: format!("constraints[{}]", i),
                                    },
                                ],
                                message: "exclude constraint requires an 'expression' field".to_string(),
                                severity: Severity::Error,
                            });
                        }
                        errors.push(ValidationError {
                            path: vec![
                                PathSegment {
                                    kind: "model".to_string(),
                                    name: model_name.to_string(),
                                },
                                PathSegment {
                                    kind: "config".to_string(),
                                    name: format!("constraints[{}]", i),
                                },
                            ],
                            message: "exclude constraint is only supported for PostgreSQL dialect".to_string(),
                            severity: Severity::Warning,
                        });
                    }
                    Some("foreign_key") => {
                        if constraint.get("reference").is_none() {
                            errors.push(ValidationError {
                                path: vec![
                                    PathSegment {
                                        kind: "model".to_string(),
                                        name: model_name.to_string(),
                                    },
                                    PathSegment {
                                        kind: "config".to_string(),
                                        name: format!("constraints[{}]", i),
                                    },
                                ],
                                message: "foreign_key constraint requires a 'reference' field".to_string(),
                                severity: Severity::Error,
                            });
                        } else if let Some(reference) = constraint.get("reference") {
                            validate_reference(reference, model_name, &format!("constraints[{}].reference", i), errors);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn validate_field_config(
    config: &JSON,
    model_name: &str,
    field_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate references
    if let Some(reference) = config.get("references") {
        validate_reference(reference, model_name, field_name, errors);
    }

    // Validate relationship
    if let Some(relationship) = config.get("relationship") {
        validate_relationship(relationship, model_name, field_name, errors);
    }
}

fn validate_reference(
    reference: &JSON,
    model_name: &str,
    field_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate table is required and non-empty
    if let Some(table) = reference.get("table") {
        if let Some(table_str) = table.as_str() {
            if table_str.is_empty() {
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
                            name: "references.table".to_string(),
                        },
                    ],
                    message: "reference table cannot be empty".to_string(),
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
                    name: "references".to_string(),
                },
            ],
            message: "reference must have a 'table' field".to_string(),
            severity: Severity::Error,
        });
    }

    // Validate on_delete action
    if let Some(on_delete) = reference.get("on_delete") {
        if let Some(action) = on_delete.as_str() {
            if !["cascade", "set_null", "restrict", "no_action", "set_default"].contains(&action) {
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
                            name: "references.on_delete".to_string(),
                        },
                    ],
                    message: "on_delete must be 'cascade', 'set_null', 'restrict', 'no_action', or 'set_default'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate on_update action
    if let Some(on_update) = reference.get("on_update") {
        if let Some(action) = on_update.as_str() {
            if !["cascade", "set_null", "restrict", "no_action", "set_default"].contains(&action) {
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
                            name: "references.on_update".to_string(),
                        },
                    ],
                    message: "on_update must be 'cascade', 'set_null', 'restrict', 'no_action', or 'set_default'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }
}

fn validate_relationship(
    relationship: &JSON,
    model_name: &str,
    field_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate relationship type is required
    if let Some(rel_type) = relationship.get("type") {
        if let Some(type_str) = rel_type.as_str() {
            if !["one_to_one", "one_to_many", "many_to_many"].contains(&type_str) {
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
                            name: "relationship.type".to_string(),
                        },
                    ],
                    message: "relationship type must be 'one_to_one', 'one_to_many', or 'many_to_many'".to_string(),
                    severity: Severity::Error,
                });
            }

            // Validate many_to_many requires 'through' field
            if type_str == "many_to_many" {
                if let Some(through) = relationship.get("through") {
                    if let Some(through_str) = through.as_str() {
                        if through_str.is_empty() {
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
                                        name: "relationship.through".to_string(),
                                    },
                                ],
                                message: "through table name cannot be empty".to_string(),
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
                                name: "relationship".to_string(),
                            },
                        ],
                        message: "many_to_many relationship requires a 'through' field".to_string(),
                        severity: Severity::Error,
                    });
                }
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
                    name: "relationship".to_string(),
                },
            ],
            message: "relationship must have a 'type' field".to_string(),
            severity: Severity::Error,
        });
    }
}


#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;
