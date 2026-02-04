use crate::build::build;
use cdm_plugin_interface::{
    FieldDefinition, ModelDefinition, Schema, TypeExpression, Utils,
};
use serde_json::json;
use std::collections::HashMap;

fn utils() -> Utils {
    Utils
}

fn create_test_schema() -> Schema {
    let mut models = HashMap::new();
    let type_aliases = HashMap::new();

    // User model
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            config: json!({}),
            entity_id: None,
        },
    );

    // GetUserInput model
    models.insert(
        "GetUserInput".to_string(),
        ModelDefinition {
            name: "GetUserInput".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: None,
            }],
            config: json!({}),
            entity_id: None,
        },
    );

    // CreateUserInput model
    models.insert(
        "CreateUserInput".to_string(),
        ModelDefinition {
            name: "CreateUserInput".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            config: json!({}),
            entity_id: None,
        },
    );

    // DeleteUserInput model
    models.insert(
        "DeleteUserInput".to_string(),
        ModelDefinition {
            name: "DeleteUserInput".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: None,
            }],
            config: json!({}),
            entity_id: None,
        },
    );

    // ValidationError model
    models.insert(
        "ValidationError".to_string(),
        ModelDefinition {
            name: "ValidationError".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "code".to_string(),
                    field_type: TypeExpression::StringLiteral {
                        value: "VALIDATION_ERROR".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "message".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases,
    }
}

// ============================================================================
// Basic Build Tests
// ============================================================================

#[test]
fn test_build_generates_single_file() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "input": "GetUserInput",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "contract.ts");
}

#[test]
fn test_build_includes_header_comment() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("Generated by CDM @trpc plugin"));
    assert!(content.contains("DO NOT EDIT"));
}

#[test]
fn test_build_imports_trpc() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("import { initTRPC } from '@trpc/server'"));
}

#[test]
fn test_build_imports_zod_when_needed_for_void() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "deleteUser": {
                "type": "mutation",
                "input": "DeleteUserInput",
                "output": "void"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(
        content.contains("import { z } from 'zod'"),
        "Should import zod for z.void()"
    );
}

#[test]
fn test_build_imports_zod_when_needed_for_array() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "listUsers": {
                "type": "query",
                "output": "User[]"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(
        content.contains("import { z } from 'zod'"),
        "Should import zod for z.array()"
    );
}

#[test]
fn test_build_no_zod_import_for_single_model_outputs() {
    // When all outputs are single models (no void, no arrays), zod is not needed
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "input": "GetUserInput",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(
        !content.contains("import { z } from 'zod'"),
        "Should NOT import zod when not needed. Content:\n{}",
        content
    );
}

#[test]
fn test_build_imports_observable_for_subscriptions() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "onUserCreated": {
                "type": "subscription",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("import { initTRPC, TRPCError } from '@trpc/server'"));
    assert!(content.contains("import { observable, type Observable } from '@trpc/server/observable'"));
}

#[test]
fn test_build_no_observable_import_without_subscriptions() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(!content.contains("observable"));
}

#[test]
fn test_build_imports_schemas() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "input": "GetUserInput",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("GetUserInputSchema"));
    assert!(content.contains("UserSchema"));
    assert!(content.contains("from './types'"));
}

// ============================================================================
// Procedure Generation Tests
// ============================================================================

#[test]
fn test_build_generates_query_procedure() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "input": "GetUserInput",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("getUser: publicProcedure"));
    assert!(content.contains(".input(GetUserInputSchema)"));
    assert!(content.contains(".output(UserSchema)"));
    assert!(content.contains(".query("));
}

#[test]
fn test_build_generates_mutation_procedure() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "createUser": {
                "type": "mutation",
                "input": "CreateUserInput",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("createUser: publicProcedure"));
    assert!(content.contains(".input(CreateUserInputSchema)"));
    assert!(content.contains(".output(UserSchema)"));
    assert!(content.contains(".mutation("));
}

#[test]
fn test_build_generates_subscription_procedure() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "onUserCreated": {
                "type": "subscription",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("onUserCreated: publicProcedure"));
    assert!(content.contains(".output(UserSchema)"));
    assert!(content.contains(".subscription("));
    assert!(content.contains("observable<User>"));
}

#[test]
fn test_build_generates_procedure_without_input() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "listUsers": {
                "type": "query",
                "output": "User[]"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("listUsers: publicProcedure"));
    assert!(!content.contains(".input("));
    // Stub has no parameters since they're unused
    assert!(content.contains(".query((): never =>"));
}

#[test]
fn test_build_generates_procedure_with_input() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "input": "GetUserInput",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should have .input() in the chain
    assert!(content.contains(".input(GetUserInputSchema)"));
    // Stub has no parameters since they're unused
    assert!(content.contains(".query((): never =>"));
}

// ============================================================================
// Output Type Tests
// ============================================================================

#[test]
fn test_build_generates_single_output() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains(".output(UserSchema)"));
}

#[test]
fn test_build_generates_array_output() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "listUsers": {
                "type": "query",
                "output": "User[]"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains(".output(z.array(UserSchema))"));
    assert!(content.contains("return User[]"));
}

#[test]
fn test_build_generates_void_output() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "deleteUser": {
                "type": "mutation",
                "input": "DeleteUserInput",
                "output": "void"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains(".output(z.void())"));
    assert!(content.contains("return void"));
}

// ============================================================================
// Router Structure Tests
// ============================================================================

#[test]
fn test_build_creates_router() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("const t = initTRPC.context<TContext>().create()"));
    assert!(content.contains("const router = t.router"));
    assert!(content.contains("const publicProcedure = t.procedure"));
    assert!(content.contains("export const appRouter = router({"));
    assert!(content.contains("export type AppRouter = typeof appRouter"));
}

#[test]
fn test_build_with_multiple_procedures() {
    let schema = create_test_schema();
    let config = json!({
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
                "output": "User"
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Verify all procedures are generated
    assert!(content.contains("getUser:"));
    assert!(content.contains("listUsers:"));
    assert!(content.contains("createUser:"));
    assert!(content.contains("deleteUser:"));
    assert!(content.contains("onUserCreated:"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_build_with_empty_procedures() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {}
    });

    let files = build(schema, config, &utils());
    assert!(files.is_empty());
}

#[test]
fn test_build_without_procedures() {
    let schema = create_test_schema();
    let config = json!({});

    let files = build(schema, config, &utils());
    assert!(files.is_empty());
}

// ============================================================================
// Schema Import Configuration Tests
// ============================================================================

#[test]
fn test_build_default_schema_import_path() {
    // Default behavior: imports from './types'
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("from './types'"));
}

#[test]
fn test_build_custom_schema_import_single_strategy() {
    // Custom single file path for schema imports
    let schema = create_test_schema();
    let config = json!({
        "schema_import": {
            "strategy": "single",
            "path": "./generated/schemas"
        },
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("from './generated/schemas'"));
    assert!(!content.contains("from './types'"));
}

#[test]
fn test_build_schema_import_per_model_strategy() {
    // Per-model strategy: imports from individual files in directory
    let schema = create_test_schema();
    let config = json!({
        "schema_import": {
            "strategy": "per_model",
            "path": "./models"
        },
        "procedures": {
            "getUser": {
                "type": "query",
                "input": "GetUserInput",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should import from individual model files
    assert!(content.contains("from './models/User'"));
    assert!(content.contains("from './models/GetUserInput'"));
    // Should NOT have a single types import
    assert!(!content.contains("from './types'"));
}

#[test]
fn test_per_model_strategy_imports_typescript_type_for_subscriptions() {
    // Per-model strategy with subscriptions should import both schema and type
    let schema = create_test_schema();
    let config = json!({
        "schema_import": {
            "strategy": "per_model",
            "path": "./models"
        },
        "procedures": {
            "onUserCreated": {
                "type": "subscription",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should import both the TypeScript type and schema from the model file
    assert!(
        content.contains("type User, UserSchema"),
        "Per-model strategy should import both type and schema. Content:\n{}",
        content
    );
    assert!(content.contains("from './models/User'"));
}

// ============================================================================
// Subscription Handler Tests
// ============================================================================

#[test]
fn test_subscription_generates_observable_pattern() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "onUserCreated": {
                "type": "subscription",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("return observable<User>(_emit =>"));
    assert!(content.contains("_emit.next(value)"));
    assert!(content.contains("return () => { /* cleanup */ }"));
}

#[test]
fn test_subscription_imports_typescript_type_for_observable() {
    // Bug fix test: subscription procedures use observable<T> which requires
    // the TypeScript type to be imported, not just the Zod schema.
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "onUserCreated": {
                "type": "subscription",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should import both the Zod schema AND the TypeScript type
    assert!(content.contains("UserSchema"), "Should import UserSchema for .output()");
    assert!(
        content.contains("type User,") || content.contains("type User\n"),
        "Should import TypeScript type User for observable<User>. Content:\n{}",
        content
    );
}

#[test]
fn test_subscription_array_output_imports_typescript_type() {
    // Bug fix test: array subscription outputs like User[] should import
    // the base TypeScript type (User) for observable<User[]>
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "onUsersUpdated": {
                "type": "subscription",
                "output": "User[]"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should import both the Zod schema AND the TypeScript type
    assert!(content.contains("UserSchema"), "Should import UserSchema for .output()");
    assert!(
        content.contains("type User,") || content.contains("type User\n"),
        "Should import TypeScript type User for observable<User[]>. Content:\n{}",
        content
    );
}

#[test]
fn test_query_does_not_import_typescript_type() {
    // Query procedures don't need explicit TypeScript type imports
    // because types are inferred from Zod schemas
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should only import the Zod schema, NOT the TypeScript type
    assert!(content.contains("UserSchema"), "Should import UserSchema for .output()");
    assert!(
        !content.contains("type User"),
        "Should NOT import TypeScript type for query procedures. Content:\n{}",
        content
    );
}

#[test]
fn test_mixed_procedures_only_import_types_for_subscriptions() {
    // When we have both query and subscription, only subscription outputs
    // should have TypeScript type imports
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "input": "GetUserInput",
                "output": "User"
            },
            "onValidationError": {
                "type": "subscription",
                "output": "ValidationError"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should import ValidationError as a type (for subscription)
    assert!(
        content.contains("type ValidationError"),
        "Should import TypeScript type ValidationError for subscription. Content:\n{}",
        content
    );
    // Should NOT import User as a type (only used in query)
    assert!(
        !content.contains("type User"),
        "Should NOT import TypeScript type User (only used in query). Content:\n{}",
        content
    );
}

#[test]
fn test_subscription_with_array_output() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "onUsersUpdated": {
                "type": "subscription",
                "output": "User[]"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("return observable<User[]>(_emit =>"));
}

// ============================================================================
// Not Implemented Handler Tests
// ============================================================================

#[test]
fn test_query_includes_not_implemented() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("throw new Error('Not implemented')"));
}

#[test]
fn test_mutation_includes_not_implemented() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "createUser": {
                "type": "mutation",
                "input": "CreateUserInput",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("throw new Error('Not implemented')"));
}

// ============================================================================
// Nested Router Tests (dotted procedure names)
// ============================================================================

#[test]
fn test_build_dotted_procedure_name_generates_nested_router() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "auth.getUser": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should generate nested router, not bare dotted key
    assert!(content.contains("auth: router({"));
    assert!(content.contains("getUser: publicProcedure"));
    // Should NOT contain invalid bare dotted key
    assert!(!content.contains("auth.getUser: publicProcedure"));
}

#[test]
fn test_build_multiple_procedures_same_namespace() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "auth.getUser": {
                "type": "query",
                "output": "User"
            },
            "auth.createUser": {
                "type": "mutation",
                "input": "CreateUserInput",
                "output": "User"
            },
            "auth.deleteUser": {
                "type": "mutation",
                "input": "DeleteUserInput",
                "output": "void"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // All auth procedures should be in the same nested router
    assert!(content.contains("auth: router({"));
    assert!(content.contains("getUser: publicProcedure"));
    assert!(content.contains("createUser: publicProcedure"));
    assert!(content.contains("deleteUser: publicProcedure"));
    // Should NOT contain invalid bare dotted keys
    assert!(!content.contains("auth.getUser:"));
    assert!(!content.contains("auth.createUser:"));
    assert!(!content.contains("auth.deleteUser:"));
}

#[test]
fn test_build_mixed_namespaced_and_flat_procedures() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "health": {
                "type": "query",
                "output": "User"
            },
            "auth.getUser": {
                "type": "query",
                "output": "User"
            },
            "users.list": {
                "type": "query",
                "output": "User[]"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Flat procedure
    assert!(content.contains("health: publicProcedure"));
    // Nested routers
    assert!(content.contains("auth: router({"));
    assert!(content.contains("users: router({"));
    // Nested procedures
    assert!(content.contains("getUser: publicProcedure"));
    assert!(content.contains("list: publicProcedure"));
}

#[test]
fn test_build_deeply_nested_procedure_name() {
    let schema = create_test_schema();
    let config = json!({
        "procedures": {
            "api.v1.users.get": {
                "type": "query",
                "output": "User"
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should generate deeply nested routers
    assert!(content.contains("api: router({"));
    assert!(content.contains("v1: router({"));
    assert!(content.contains("users: router({"));
    assert!(content.contains("get: publicProcedure"));
    // Should NOT contain invalid bare dotted key
    assert!(!content.contains("api.v1.users.get:"));
}

