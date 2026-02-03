use cdm_plugin_interface::{OutputFile, Schema, TypeExpression, Utils, JSON};

/// Builds documentation from the schema
pub fn build(schema: Schema, config: JSON, utils: &Utils) -> Vec<OutputFile> {
    let format = config
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("markdown");

    match format {
        "markdown" => build_markdown(schema, config, utils),
        "html" => build_html(schema, config, utils),
        "json" => build_json(schema, config, utils),
        _ => vec![],
    }
}

fn build_markdown(schema: Schema, config: JSON, _utils: &Utils) -> Vec<OutputFile> {
    let mut content = String::new();

    // Title
    if let Some(title) = config.get("title").and_then(|v| v.as_str()) {
        content.push_str(&format!("# {}\n\n", title));
    } else {
        content.push_str("# Schema Documentation\n\n");
    }

    // Table of contents
    content.push_str("## Table of Contents\n\n");

    if !schema.type_aliases.is_empty() {
        content.push_str("### Type Aliases\n\n");
        for (name, alias) in schema.type_aliases.iter() {
            // Skip hidden type aliases in TOC
            if alias.config.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }
            content.push_str(&format!("- [{}](#type-{})\n", name, name.to_lowercase()));
        }
        content.push('\n');
    }

    if !schema.models.is_empty() {
        content.push_str("### Models\n\n");
        for (name, model) in schema.models.iter() {
            // Skip hidden models in TOC
            if model.config.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }
            content.push_str(&format!("- [{}](#model-{})\n", name, name.to_lowercase()));
        }
        content.push('\n');
    }

    // Type Aliases section
    if !schema.type_aliases.is_empty() {
        content.push_str("## Type Aliases\n\n");

        for (name, alias) in schema.type_aliases.iter() {
            // Skip hidden types
            if alias.config.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }

            content.push_str(&format!("### Type: {}\n\n", name));

            // Description
            if let Some(desc) = alias.config.get("description").and_then(|v| v.as_str()) {
                content.push_str(&format!("{}\n\n", desc));
            }

            // Type definition
            content.push_str(&format!("**Type:** `{}`\n\n", format_type_expression(&alias.alias_type)));

            // Example
            if config.get("include_examples").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(example) = alias.config.get("example").and_then(|v| v.as_str()) {
                    content.push_str("**Example:**\n\n");
                    content.push_str(&format!("```\n{}\n```\n\n", example));
                }
            }

            content.push_str("---\n\n");
        }
    }

    // Models section
    if !schema.models.is_empty() {
        content.push_str("## Models\n\n");

        for (name, model) in schema.models.iter() {
            // Skip hidden models
            if model.config.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }

            content.push_str(&format!("### Model: {}\n\n", name));

            // Description
            if let Some(desc) = model.config.get("description").and_then(|v| v.as_str()) {
                content.push_str(&format!("{}\n\n", desc));
            }

            // Inheritance
            if config.get("include_inheritance").and_then(|v| v.as_bool()).unwrap_or(false) && !model.parents.is_empty() {
                content.push_str("**Extends:** ");
                content.push_str(&model.parents.join(", "));
                content.push_str("\n\n");
            }

            // Fields
            if !model.fields.is_empty() {
                content.push_str("**Fields:**\n\n");
                content.push_str("| Field | Type | Required | Description |\n");
                content.push_str("|-------|------|----------|-------------|\n");

                for field in &model.fields {
                    let deprecated = field.config.get("deprecated").and_then(|v| v.as_bool()).unwrap_or(false);
                    let field_name = if deprecated {
                        format!("~~{}~~", field.name)
                    } else {
                        field.name.clone()
                    };

                    let required = if field.optional { "No" } else { "Yes" };
                    let type_str = format_type_expression(&field.field_type);
                    let description = field.config.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    content.push_str(&format!("| {} | `{}` | {} | {} |\n",
                        field_name, type_str, required, description));
                }
                content.push('\n');
            }

            // Example
            if config.get("include_examples").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(example) = model.config.get("example").and_then(|v| v.as_str()) {
                    content.push_str("**Example:**\n\n");
                    content.push_str(&format!("```json\n{}\n```\n\n", example));
                }
            }

            content.push_str("---\n\n");
        }
    }

    vec![OutputFile {
        path: "schema.md".to_string(),
        content,
    }]
}

fn build_html(schema: Schema, config: JSON, utils: &Utils) -> Vec<OutputFile> {
    // Placeholder - build markdown and wrap in HTML
    let markdown_files = build_markdown(schema, config, utils);

    let mut html_content = String::from(
        "<!DOCTYPE html>\n\
         <html>\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <title>Schema Documentation</title>\n\
         <style>\n\
         body { font-family: sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }\n\
         table { border-collapse: collapse; width: 100%; }\n\
         th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n\
         th { background-color: #f2f2f2; }\n\
         code { background-color: #f4f4f4; padding: 2px 4px; border-radius: 3px; }\n\
         pre { background-color: #f4f4f4; padding: 10px; border-radius: 5px; overflow-x: auto; }\n\
         </style>\n\
         </head>\n\
         <body>\n"
    );

    if let Some(file) = markdown_files.first() {
        // Simple markdown to HTML conversion (placeholder)
        let html_body = file.content
            .replace("# ", "<h1>")
            .replace("\n\n", "</h1>\n")
            .replace("## ", "<h2>")
            .replace("### ", "<h3>");

        html_content.push_str(&html_body);
    }

    html_content.push_str("</body>\n</html>\n");

    vec![OutputFile {
        path: "schema.html".to_string(),
        content: html_content,
    }]
}

fn build_json(schema: Schema, _config: JSON, _utils: &Utils) -> Vec<OutputFile> {
    // Build JSON representation of the schema
    let json_content = serde_json::to_string_pretty(&schema).unwrap_or_default();

    vec![OutputFile {
        path: "schema.json".to_string(),
        content: json_content,
    }]
}

fn format_type_expression(type_expr: &TypeExpression) -> String {
    match type_expr {
        TypeExpression::Identifier { name } => name.clone(),
        TypeExpression::Array { element_type } => {
            format!("{}[]", format_type_expression(element_type))
        }
        TypeExpression::Map { value_type, key_type } => {
            format!(
                "{}[{}]",
                format_type_expression(value_type),
                format_type_expression(key_type)
            )
        }
        TypeExpression::Union { types } => {
            types
                .iter()
                .map(|t| format_type_expression(t))
                .collect::<Vec<_>>()
                .join(" | ")
        }
        TypeExpression::StringLiteral { value } => {
            format!("\"{}\"", value)
        }
        TypeExpression::NumberLiteral { value } => {
            if value.fract() == 0.0 {
                format!("{}", *value as i64)
            } else {
                format!("{}", value)
            }
        }
    }
}

#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
