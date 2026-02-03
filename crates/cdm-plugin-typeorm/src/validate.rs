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

    // Validate definite_assignment is a boolean if specified
    if let Some(definite_assignment) = config.get("definite_assignment") {
        if !definite_assignment.is_boolean() {
            errors.push(ValidationError {
                path: vec![PathSegment {
                    kind: "global".to_string(),
                    name: "definite_assignment".to_string(),
                }],
                message: "definite_assignment must be a boolean".to_string(),
                severity: Severity::Error,
            });
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

    // Validate ts_type if present
    if let Some(ts_type) = config.get("ts_type") {
        validate_ts_type_config(
            ts_type,
            &[
                PathSegment {
                    kind: "type_alias".to_string(),
                    name: alias_name.to_string(),
                },
                PathSegment {
                    kind: "config".to_string(),
                    name: "ts_type".to_string(),
                },
            ],
            errors,
        );
    }
}

fn validate_model_config(config: &JSON, model_name: &str, errors: &mut Vec<ValidationError>) {
    // Validate indexes (map type: Index[string], keyed by index name)
    if let Some(indexes) = config.get("indexes") {
        if let Some(indexes_obj) = indexes.as_object() {
            for (index_name, index) in indexes_obj {
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
                                        name: format!("indexes.{}", index_name),
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
                                name: format!("indexes.{}", index_name),
                            },
                        ],
                        message: "index must have a 'fields' array".to_string(),
                        severity: Severity::Error,
                    });
                }
            }
        }
    }

    // Validate hooks
    if let Some(hooks) = config.get("hooks") {
        validate_hooks(hooks, model_name, errors);
    }

    // Validate definite_assignment is a boolean if specified
    if let Some(definite_assignment) = config.get("definite_assignment") {
        if !definite_assignment.is_boolean() {
            errors.push(ValidationError {
                path: vec![
                    PathSegment {
                        kind: "model".to_string(),
                        name: model_name.to_string(),
                    },
                    PathSegment {
                        kind: "config".to_string(),
                        name: "definite_assignment".to_string(),
                    },
                ],
                message: "definite_assignment must be a boolean".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // Note: We don't warn about missing primary keys here since that validation
    // requires schema context (to check all fields). The user explicitly chose
    // "require explicit configuration" so we trust they will configure PKs.
}

fn validate_hooks(hooks: &JSON, model_name: &str, errors: &mut Vec<ValidationError>) {
    const HOOK_NAMES: &[&str] = &[
        "before_insert",
        "after_insert",
        "before_update",
        "after_update",
        "before_remove",
        "after_remove",
        "after_load",
        "before_soft_remove",
        "after_soft_remove",
        "after_recover",
    ];

    for hook_name in HOOK_NAMES {
        if let Some(hook_value) = hooks.get(*hook_name) {
            // Hook can be either a string (method name) or an object { method, import }
            if let Some(method_str) = hook_value.as_str() {
                // String format: just the method name
                validate_hook_method_name(method_str, hook_name, model_name, errors);
            } else if hook_value.is_object() {
                // Object format: { method: string, import: string }
                validate_hook_config_object(hook_value, hook_name, model_name, errors);
            } else {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: format!("hooks.{}", hook_name),
                        },
                    ],
                    message: format!(
                        "{} must be a string or an object with 'method' and 'import' fields",
                        hook_name
                    ),
                    severity: Severity::Error,
                });
            }
        }
    }
}

fn validate_hook_method_name(
    method_str: &str,
    hook_name: &str,
    model_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    if method_str.is_empty() {
        errors.push(ValidationError {
            path: vec![
                PathSegment {
                    kind: "model".to_string(),
                    name: model_name.to_string(),
                },
                PathSegment {
                    kind: "config".to_string(),
                    name: format!("hooks.{}", hook_name),
                },
            ],
            message: format!("{} method name cannot be empty", hook_name),
            severity: Severity::Error,
        });
    } else if !is_valid_identifier(method_str) {
        errors.push(ValidationError {
            path: vec![
                PathSegment {
                    kind: "model".to_string(),
                    name: model_name.to_string(),
                },
                PathSegment {
                    kind: "config".to_string(),
                    name: format!("hooks.{}", hook_name),
                },
            ],
            message: format!("{} must be a valid JavaScript identifier", hook_name),
            severity: Severity::Error,
        });
    }
}

fn validate_hook_config_object(
    hook_config: &JSON,
    hook_name: &str,
    model_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate 'method' field
    if let Some(method) = hook_config.get("method") {
        if let Some(method_str) = method.as_str() {
            if method_str.is_empty() {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: format!("hooks.{}.method", hook_name),
                        },
                    ],
                    message: format!("{}.method cannot be empty", hook_name),
                    severity: Severity::Error,
                });
            } else if !is_valid_identifier(method_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: format!("hooks.{}.method", hook_name),
                        },
                    ],
                    message: format!("{}.method must be a valid JavaScript identifier", hook_name),
                    severity: Severity::Error,
                });
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
                        name: format!("hooks.{}.method", hook_name),
                    },
                ],
                message: format!("{}.method must be a string", hook_name),
                severity: Severity::Error,
            });
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
                    name: format!("hooks.{}", hook_name),
                },
            ],
            message: format!("{} object must have a 'method' field", hook_name),
            severity: Severity::Error,
        });
    }

    // Validate 'import' field
    if let Some(import_path) = hook_config.get("import") {
        if let Some(import_str) = import_path.as_str() {
            if import_str.is_empty() {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: format!("hooks.{}.import", hook_name),
                        },
                    ],
                    message: format!("{}.import cannot be empty", hook_name),
                    severity: Severity::Error,
                });
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
                        name: format!("hooks.{}.import", hook_name),
                    },
                ],
                message: format!("{}.import must be a string", hook_name),
                severity: Severity::Error,
            });
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
                    name: format!("hooks.{}", hook_name),
                },
            ],
            message: format!("{} object must have an 'import' field", hook_name),
            severity: Severity::Error,
        });
    }
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

    // Validate field-level join_column
    if let Some(join_column) = config.get("join_column") {
        validate_field_level_join_column(join_column, model_name, field_name, config.get("relation").is_some(), errors);
    }

    // Validate field-level join_table
    if let Some(join_table) = config.get("join_table") {
        validate_field_level_join_table(join_table, model_name, field_name, config.get("relation").is_some(), errors);
    }

    // Validate ts_type config
    if let Some(ts_type) = config.get("ts_type") {
        validate_ts_type_config(
            ts_type,
            &[
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
                    name: "ts_type".to_string(),
                },
            ],
            errors,
        );
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

    // Validate definite_assignment is a boolean if specified
    if let Some(definite_assignment) = config.get("definite_assignment") {
        if !definite_assignment.is_boolean() {
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
                        name: "definite_assignment".to_string(),
                    },
                ],
                message: "definite_assignment must be a boolean".to_string(),
                severity: Severity::Error,
            });
        }
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

/// Validates field-level join_column configuration
fn validate_field_level_join_column(
    join_column: &JSON,
    model_name: &str,
    field_name: &str,
    has_relation: bool,
    errors: &mut Vec<ValidationError>,
) {
    // Warn if join_column is specified without a relation
    if !has_relation {
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
                    name: "join_column".to_string(),
                },
            ],
            message: "join_column specified without a relation configuration".to_string(),
            severity: Severity::Warning,
        });
    }

    // Validate name is a non-empty string if present
    if let Some(name) = join_column.get("name") {
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
                            name: "join_column.name".to_string(),
                        },
                    ],
                    message: "join_column.name cannot be empty".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate referenced_column is a non-empty string if present
    if let Some(ref_col) = join_column.get("referenced_column") {
        if let Some(ref_col_str) = ref_col.as_str() {
            if ref_col_str.is_empty() {
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
                            name: "join_column.referenced_column".to_string(),
                        },
                    ],
                    message: "join_column.referenced_column cannot be empty".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }
}

/// Validates field-level join_table configuration
fn validate_field_level_join_table(
    join_table: &JSON,
    model_name: &str,
    field_name: &str,
    has_relation: bool,
    errors: &mut Vec<ValidationError>,
) {
    // Warn if join_table is specified without a relation
    if !has_relation {
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
                    name: "join_table".to_string(),
                },
            ],
            message: "join_table specified without a relation configuration".to_string(),
            severity: Severity::Warning,
        });
    }

    // Validate name is required and non-empty
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
                            name: "join_table.name".to_string(),
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
                    name: "join_table".to_string(),
                },
            ],
            message: "join_table must have a 'name' field".to_string(),
            severity: Severity::Error,
        });
    }

    // Validate nested join_column if present
    if let Some(jc) = join_table.get("join_column") {
        validate_nested_join_column(jc, model_name, field_name, "join_table.join_column", errors);
    }

    // Validate nested inverse_join_column if present
    if let Some(ijc) = join_table.get("inverse_join_column") {
        validate_nested_join_column(ijc, model_name, field_name, "join_table.inverse_join_column", errors);
    }
}

/// Validates a nested join_column within join_table
fn validate_nested_join_column(
    join_column: &JSON,
    model_name: &str,
    field_name: &str,
    config_path: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate name is a non-empty string if present
    if let Some(name) = join_column.get("name") {
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
                            name: format!("{}.name", config_path),
                        },
                    ],
                    message: format!("{}.name cannot be empty", config_path),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate referenced_column is a non-empty string if present
    if let Some(ref_col) = join_column.get("referenced_column") {
        if let Some(ref_col_str) = ref_col.as_str() {
            if ref_col_str.is_empty() {
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
                            name: format!("{}.referenced_column", config_path),
                        },
                    ],
                    message: format!("{}.referenced_column cannot be empty", config_path),
                    severity: Severity::Error,
                });
            }
        }
    }
}

/// Validates ts_type config (either string or object format)
fn validate_ts_type_config(
    ts_type: &JSON,
    base_path: &[PathSegment],
    errors: &mut Vec<ValidationError>,
) {
    if let Some(type_str) = ts_type.as_str() {
        // String format: just the type name
        if type_str.is_empty() {
            errors.push(ValidationError {
                path: base_path.to_vec(),
                message: "ts_type cannot be empty".to_string(),
                severity: Severity::Error,
            });
        }
    } else if ts_type.is_object() {
        // Object format: { type, import, default? }

        // Validate 'type' field is required and non-empty
        if let Some(type_val) = ts_type.get("type") {
            if let Some(type_str) = type_val.as_str() {
                if type_str.is_empty() {
                    let mut path = base_path.to_vec();
                    path.push(PathSegment {
                        kind: "config".to_string(),
                        name: "type".to_string(),
                    });
                    errors.push(ValidationError {
                        path,
                        message: "ts_type.type cannot be empty".to_string(),
                        severity: Severity::Error,
                    });
                }
            } else {
                let mut path = base_path.to_vec();
                path.push(PathSegment {
                    kind: "config".to_string(),
                    name: "type".to_string(),
                });
                errors.push(ValidationError {
                    path,
                    message: "ts_type.type must be a string".to_string(),
                    severity: Severity::Error,
                });
            }
        } else {
            errors.push(ValidationError {
                path: base_path.to_vec(),
                message: "ts_type object must have a 'type' field".to_string(),
                severity: Severity::Error,
            });
        }

        // Validate 'import' field is required and non-empty
        if let Some(import_val) = ts_type.get("import") {
            if let Some(import_str) = import_val.as_str() {
                if import_str.is_empty() {
                    let mut path = base_path.to_vec();
                    path.push(PathSegment {
                        kind: "config".to_string(),
                        name: "import".to_string(),
                    });
                    errors.push(ValidationError {
                        path,
                        message: "ts_type.import cannot be empty".to_string(),
                        severity: Severity::Error,
                    });
                }
            } else {
                let mut path = base_path.to_vec();
                path.push(PathSegment {
                    kind: "config".to_string(),
                    name: "import".to_string(),
                });
                errors.push(ValidationError {
                    path,
                    message: "ts_type.import must be a string".to_string(),
                    severity: Severity::Error,
                });
            }
        } else {
            errors.push(ValidationError {
                path: base_path.to_vec(),
                message: "ts_type object must have an 'import' field".to_string(),
                severity: Severity::Error,
            });
        }

        // Validate 'default' field is a boolean if present
        if let Some(default_val) = ts_type.get("default") {
            if !default_val.is_boolean() {
                let mut path = base_path.to_vec();
                path.push(PathSegment {
                    kind: "config".to_string(),
                    name: "default".to_string(),
                });
                errors.push(ValidationError {
                    path,
                    message: "ts_type.default must be a boolean".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    } else {
        errors.push(ValidationError {
            path: base_path.to_vec(),
            message: "ts_type must be a string or an object with 'type' and 'import' fields"
                .to_string(),
            severity: Severity::Error,
        });
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
