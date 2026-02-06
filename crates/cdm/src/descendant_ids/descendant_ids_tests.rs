use super::*;
use crate::dependency_graph::DependencyGraph;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn create_test_project() -> TempDir {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();

    // base.cdm - no IDs assigned
    let mut base = File::create(root.join("base.cdm")).unwrap();
    writeln!(base, "User {{").unwrap();
    writeln!(base, "  id: Int").unwrap();
    writeln!(base, "  name: String").unwrap();
    writeln!(base, "}}").unwrap();

    // child.cdm - extends base, has IDs
    let mut child = File::create(root.join("src/child.cdm")).unwrap();
    writeln!(child, "extends \"../base.cdm\"").unwrap();
    writeln!(child, "").unwrap();
    writeln!(child, "Admin extends User {{").unwrap();
    writeln!(child, "  role: String #1").unwrap();
    writeln!(child, "  permissions: String[] #2").unwrap();
    writeln!(child, "}} #5").unwrap();
    writeln!(child, "").unwrap();
    writeln!(child, "EmailAddress: String #7").unwrap();

    // grandchild.cdm - extends child, has more IDs
    let mut grandchild = File::create(root.join("src/grandchild.cdm")).unwrap();
    writeln!(grandchild, "extends \"./child.cdm\"").unwrap();
    writeln!(grandchild, "").unwrap();
    writeln!(grandchild, "SuperAdmin extends Admin {{").unwrap();
    writeln!(grandchild, "  superPower: String #1").unwrap();
    writeln!(grandchild, "}} #10").unwrap();

    temp
}

#[test]
fn test_collect_global_ids_from_descendants() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();
    let descendant_ids = collect_descendant_ids(&graph, &files[0]).unwrap();

    // base.cdm's descendants use global IDs: 5 (Admin), 7 (EmailAddress), 10 (SuperAdmin)
    assert!(descendant_ids.has_global_id(5));
    assert!(descendant_ids.has_global_id(7));
    assert!(descendant_ids.has_global_id(10));

    // Should not have IDs not used
    assert!(!descendant_ids.has_global_id(1)); // field ID, not global
    assert!(!descendant_ids.has_global_id(99));
}

#[test]
fn test_collect_field_ids_from_descendants() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();
    let descendant_ids = collect_descendant_ids(&graph, &files[0]).unwrap();

    // Admin model has field IDs 1 and 2
    assert!(descendant_ids.has_field_id("Admin", 1));
    assert!(descendant_ids.has_field_id("Admin", 2));

    // SuperAdmin model has field ID 1
    assert!(descendant_ids.has_field_id("SuperAdmin", 1));

    // No field ID 3 in any model
    assert!(!descendant_ids.has_field_id("Admin", 3));
}

#[test]
fn test_collect_ids_from_child_only() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();

    // Get descendants of child.cdm (only grandchild)
    let descendant_ids = collect_descendant_ids(&graph, &files[1]).unwrap();

    // Should only have IDs from grandchild (not from child itself)
    assert!(descendant_ids.has_global_id(10)); // SuperAdmin

    // Should NOT have IDs from child.cdm
    assert!(!descendant_ids.has_global_id(5)); // Admin
    assert!(!descendant_ids.has_global_id(7)); // EmailAddress
}

#[test]
fn test_collect_ids_no_descendants() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();

    // grandchild has no descendants
    let descendant_ids = collect_descendant_ids(&graph, &files[2]).unwrap();

    assert!(descendant_ids.global_ids.is_empty());
    assert!(descendant_ids.model_field_ids.is_empty());
}

#[test]
fn test_descendant_ids_api() {
    let mut ids = DescendantIds::new();

    ids.global_ids.insert(5);
    ids.global_ids.insert(10);
    ids.model_field_ids
        .entry("User".to_string())
        .or_default()
        .insert(1);
    ids.model_field_ids
        .entry("User".to_string())
        .or_default()
        .insert(2);

    // Test global_ids accessor
    assert_eq!(ids.global_ids().len(), 2);
    assert!(ids.global_ids().contains(&5));

    // Test field_ids_for_model accessor
    let user_fields = ids.field_ids_for_model("User").unwrap();
    assert_eq!(user_fields.len(), 2);
    assert!(user_fields.contains(&1));

    // Non-existent model
    assert!(ids.field_ids_for_model("NonExistent").is_none());
}
