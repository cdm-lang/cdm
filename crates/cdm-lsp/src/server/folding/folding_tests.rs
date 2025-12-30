use super::*;

#[test]
fn test_model_folding() {
    let text = r#"User {
  name: string #1
  email: string #2
  age: number #3
} #10"#;

    let ranges = compute_folding_ranges(text).unwrap();

    // Should have one folding range for the model body
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].start_line, 0); // Line with opening brace
    assert_eq!(ranges[0].end_line, 4); // Line with closing brace
    assert_eq!(ranges[0].kind, Some(FoldingRangeKind::Region));
}

#[test]
fn test_nested_folding() {
    let text = r#"User {
  name: string {
@sql { type: "VARCHAR(255)" }
  } #1
} #10"#;

    let ranges = compute_folding_ranges(text).unwrap();

    // Should have folding ranges for:
    // 1. Model body
    // 2. Plugin block
    // 3. Object literal
    assert!(ranges.len() >= 2);
}

#[test]
fn test_multiple_models_folding() {
    let text = r#"User {
  name: string #1
} #10

Admin extends User {
  level: number #1
} #11

Post {
  title: string #1
  content: string #2
} #12"#;

    let ranges = compute_folding_ranges(text).unwrap();

    // Should have three folding ranges (one for each model body)
    assert_eq!(ranges.len(), 3);
}

#[test]
fn test_single_line_no_folding() {
    let text = "Email: string #1";

    let ranges = compute_folding_ranges(text);

    // Should have no folding ranges for single-line content
    assert!(ranges.is_none() || ranges.unwrap().is_empty());
}

#[test]
fn test_plugin_config_folding() {
    let text = r#"@sql {
  dialect: "postgres",
  schema: "public"
}

User {
  name: string #1
} #10"#;

    let ranges = compute_folding_ranges(text).unwrap();

    // Should have folding ranges for:
    // 1. Plugin config object
    // 2. Model body
    assert!(ranges.len() >= 2);
}
