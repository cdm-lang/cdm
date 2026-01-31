use crate::validate::{
    collect_model_references, is_array_output, is_void_output, strip_array_suffix, validate_config,
};
use cdm_plugin_interface::{ConfigLevel, Severity, Utils};
use serde_json::json;

fn utils() -> Utils {
    Utils
}

// ============================================================================
// Schema Import Validation Tests
// ============================================================================

#[test]
fn test_schema_import_valid_single_strategy() {
    let config = json!({
        "schema_import": {
            "strategy": "single",
            "path": "./types"
        },
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("schema_import"));
    assert!(error.is_none(), "Expected no schema_import errors");
}

#[test]
fn test_schema_import_valid_per_model_strategy() {
    let config = json!({
        "schema_import": {
            "strategy": "per_model",
            "path": "./models"
        },
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("schema_import"));
    assert!(error.is_none(), "Expected no schema_import errors");
}

#[test]
fn test_schema_import_invalid_strategy() {
    let config = json!({
        "schema_import": {
            "strategy": "invalid",
            "path": "./types"
        },
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("invalid strategy") && e.message.contains("invalid"));
    assert!(error.is_some(), "Expected error for invalid strategy");
}

#[test]
fn test_schema_import_missing_path() {
    let config = json!({
        "schema_import": {
            "strategy": "single"
        },
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("schema_import.path"));
    assert!(error.is_some(), "Expected error for missing path");
}

#[test]
fn test_schema_import_missing_strategy() {
    let config = json!({
        "schema_import": {
            "path": "./types"
        },
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("schema_import.strategy"));
    assert!(error.is_some(), "Expected error for missing strategy");
}

// ============================================================================
// Array/Void Output Helper Tests
// ============================================================================

#[test]
fn test_strip_array_suffix() {
    assert_eq!(strip_array_suffix("User[]"), "User");
    assert_eq!(strip_array_suffix("User"), "User");
    assert_eq!(strip_array_suffix("ValidationError[]"), "ValidationError");
}

#[test]
fn test_is_array_output() {
    assert!(is_array_output("User[]"));
    assert!(!is_array_output("User"));
    assert!(!is_array_output("void"));
}

#[test]
fn test_is_void_output() {
    assert!(is_void_output("void"));
    assert!(!is_void_output("User"));
    assert!(!is_void_output("User[]"));
}

// ============================================================================
// Model Reference Collection Tests
// ============================================================================

#[test]
fn test_collect_model_references() {
    let procedures = json!({
        "getUser": {
            "type": "query",
            "input": "GetUserInput",
            "output": "User"
        },
        "listUsers": {
            "type": "query",
            "output": "User[]"
        },
        "createUser": {
            "type": "mutation",
            "input": "CreateUserInput",
            "output": "User",
            "error": "ValidationError"
        },
        "deleteUser": {
            "type": "mutation",
            "input": "DeleteUserInput",
            "output": "void"
        }
    });

    let refs = collect_model_references(&procedures);

    assert!(refs.contains("GetUserInput"));
    assert!(refs.contains("User"));
    assert!(refs.contains("CreateUserInput"));
    assert!(refs.contains("ValidationError"));
    assert!(refs.contains("DeleteUserInput"));
    // void should not be collected as a model reference
    assert!(!refs.contains("void"));
}

#[test]
fn test_collect_model_references_handles_arrays() {
    let procedures = json!({
        "listItems": {
            "type": "query",
            "output": "Item[]"
        }
    });

    let refs = collect_model_references(&procedures);

    // Should collect "Item" not "Item[]"
    assert!(refs.contains("Item"));
    assert!(!refs.contains("Item[]"));
}

// ============================================================================
// Procedures Required Validation
// ============================================================================

#[test]
fn test_procedures_required() {
    let config = json!({});

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("procedures is required"));
    assert!(error.is_some(), "Expected procedures required error");
}

#[test]
fn test_procedures_empty() {
    let config = json!({
        "procedures": {}
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("procedures must contain at least one procedure"));
    assert!(error.is_some(), "Expected error for empty procedures");
}

#[test]
fn test_procedures_must_be_object() {
    let config = json!({
        "procedures": "not an object"
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("procedures must be an object"));
    assert!(error.is_some(), "Expected error for non-object procedures");
}

// ============================================================================
// Procedure Type Validation
// ============================================================================

#[test]
fn test_procedure_type_required() {
    let config = json!({
        "procedures": {
            "getUser": {
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("missing required field 'type'"));
    assert!(error.is_some(), "Expected type required error");
}

#[test]
fn test_procedure_type_invalid() {
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "invalid",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors.iter().find(|e| e.message.contains("invalid procedure type"));
    assert!(error.is_some(), "Expected invalid type error");
}

#[test]
fn test_procedure_type_valid_query() {
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(actual_errors.is_empty(), "Expected no errors for valid query");
}

#[test]
fn test_procedure_type_valid_mutation() {
    let config = json!({
        "procedures": {
            "createUser": {
                "type": "mutation",
                "input": "CreateUserInput",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(actual_errors.is_empty(), "Expected no errors for valid mutation");
}

#[test]
fn test_procedure_type_valid_subscription() {
    let config = json!({
        "procedures": {
            "onUserCreated": {
                "type": "subscription",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(actual_errors.is_empty(), "Expected no errors for valid subscription");
}

// ============================================================================
// Procedure Output Validation
// ============================================================================

#[test]
fn test_procedure_output_required() {
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("missing required field 'output'"));
    assert!(error.is_some(), "Expected output required error");
}

#[test]
fn test_procedure_output_must_be_string() {
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": 123
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors.iter().find(|e| e.message.contains("output must be a string"));
    assert!(error.is_some(), "Expected output type error");
}

#[test]
fn test_procedure_output_void_valid() {
    let config = json!({
        "procedures": {
            "deleteUser": {
                "type": "mutation",
                "input": "DeleteUserInput",
                "output": "void"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(actual_errors.is_empty(), "Expected no errors for void output");
}

#[test]
fn test_procedure_output_array_valid() {
    let config = json!({
        "procedures": {
            "listUsers": {
                "type": "query",
                "output": "User[]"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(actual_errors.is_empty(), "Expected no errors for array output");
}

// ============================================================================
// Procedure Input Validation
// ============================================================================

#[test]
fn test_procedure_input_must_be_string() {
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "input": 123,
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors.iter().find(|e| e.message.contains("input must be a string"));
    assert!(error.is_some(), "Expected input type error");
}

#[test]
fn test_procedure_input_optional() {
    let config = json!({
        "procedures": {
            "listUsers": {
                "type": "query",
                "output": "User[]"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(actual_errors.is_empty(), "Input should be optional");
}

// ============================================================================
// Procedure Error Validation
// ============================================================================

#[test]
fn test_procedure_error_must_be_string() {
    let config = json!({
        "procedures": {
            "createUser": {
                "type": "mutation",
                "input": "CreateUserInput",
                "output": "User",
                "error": 123
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors.iter().find(|e| e.message.contains("error must be a string"));
    assert!(error.is_some(), "Expected error type error");
}

#[test]
fn test_procedure_error_optional() {
    let config = json!({
        "procedures": {
            "createUser": {
                "type": "mutation",
                "input": "CreateUserInput",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(actual_errors.is_empty(), "Error should be optional");
}

// ============================================================================
// Valid Complete Configuration
// ============================================================================

#[test]
fn test_valid_complete_config() {
    let config = json!({
        "schema_import": {
            "strategy": "single",
            "path": "./types"
        },
        "procedures": {
            "getUser": {
                "type": "query",
                "input": "GetUserInput",
                "output": "User"
            },
            "listUsers": {
                "type": "query",
                "output": "User[]"
            },
            "createUser": {
                "type": "mutation",
                "input": "CreateUserInput",
                "output": "User",
                "error": "ValidationError"
            },
            "deleteUser": {
                "type": "mutation",
                "input": "DeleteUserInput",
                "output": "void"
            },
            "onUserCreated": {
                "type": "subscription",
                "output": "User"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(
        actual_errors.is_empty(),
        "Expected no errors for valid config, got: {:?}",
        actual_errors
    );
}

// ============================================================================
// ConfigLevel Tests
// ============================================================================

#[test]
fn test_type_alias_level_returns_empty() {
    let config = json!({});
    let errors = validate_config(
        ConfigLevel::TypeAlias {
            name: "TestAlias".to_string(),
        },
        config,
        &utils(),
    );
    assert!(errors.is_empty());
}

#[test]
fn test_model_level_returns_empty() {
    let config = json!({});
    let errors = validate_config(
        ConfigLevel::Model {
            name: "TestModel".to_string(),
        },
        config,
        &utils(),
    );
    assert!(errors.is_empty());
}

#[test]
fn test_field_level_returns_empty() {
    let config = json!({});
    let errors = validate_config(
        ConfigLevel::Field {
            model: "TestModel".to_string(),
            field: "testField".to_string(),
        },
        config,
        &utils(),
    );
    assert!(errors.is_empty());
}
