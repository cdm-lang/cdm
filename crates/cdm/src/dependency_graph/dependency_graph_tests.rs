use super::*;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn create_test_project() -> TempDir {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create directory structure
    fs::create_dir_all(root.join("src")).unwrap();

    // base.cdm - no extends
    let mut base = File::create(root.join("base.cdm")).unwrap();
    writeln!(base, "User {{").unwrap();
    writeln!(base, "  id: Int").unwrap();
    writeln!(base, "}}").unwrap();

    // child.cdm - extends base.cdm
    let mut child = File::create(root.join("src/child.cdm")).unwrap();
    writeln!(child, "extends \"../base.cdm\"").unwrap();
    writeln!(child, "").unwrap();
    writeln!(child, "Admin extends User {{").unwrap();
    writeln!(child, "  role: String").unwrap();
    writeln!(child, "}}").unwrap();

    // grandchild.cdm - extends child.cdm
    let mut grandchild = File::create(root.join("src/grandchild.cdm")).unwrap();
    writeln!(grandchild, "extends \"./child.cdm\"").unwrap();
    writeln!(grandchild, "").unwrap();
    writeln!(grandchild, "SuperAdmin extends Admin {{").unwrap();
    writeln!(grandchild, "  superPower: String").unwrap();
    writeln!(grandchild, "}}").unwrap();

    // sibling.cdm - also extends base.cdm (different branch)
    let mut sibling = File::create(root.join("src/sibling.cdm")).unwrap();
    writeln!(sibling, "extends \"../base.cdm\"").unwrap();
    writeln!(sibling, "").unwrap();
    writeln!(sibling, "Guest extends User {{").unwrap();
    writeln!(sibling, "  isGuest: Boolean").unwrap();
    writeln!(sibling, "}}").unwrap();

    // standalone.cdm - no extends
    let mut standalone = File::create(root.join("standalone.cdm")).unwrap();
    writeln!(standalone, "Config {{").unwrap();
    writeln!(standalone, "  setting: String").unwrap();
    writeln!(standalone, "}}").unwrap();

    temp
}

#[test]
fn test_build_graph_from_files() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
        root.join("src/sibling.cdm").canonicalize().unwrap(),
        root.join("standalone.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();

    // base.cdm should have 2 direct dependents: child.cdm and sibling.cdm
    let base_dependents = graph.direct_dependents(&files[0]);
    assert_eq!(base_dependents.len(), 2);

    // child.cdm should have 1 direct dependent: grandchild.cdm
    let child_dependents = graph.direct_dependents(&files[1]);
    assert_eq!(child_dependents.len(), 1);
    assert!(child_dependents.contains(&files[2]));

    // grandchild.cdm should have no dependents
    let grandchild_dependents = graph.direct_dependents(&files[2]);
    assert!(grandchild_dependents.is_empty());

    // standalone.cdm should have no dependents
    let standalone_dependents = graph.direct_dependents(&files[4]);
    assert!(standalone_dependents.is_empty());
}

#[test]
fn test_get_all_dependents_transitive() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
        root.join("src/sibling.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();

    // base.cdm should have 3 total dependents: child, grandchild, sibling
    let all_dependents = graph.get_all_dependents(&files[0]);
    assert_eq!(all_dependents.len(), 3);
    assert!(all_dependents.contains(&files[1])); // child
    assert!(all_dependents.contains(&files[2])); // grandchild
    assert!(all_dependents.contains(&files[3])); // sibling
}

#[test]
fn test_get_all_dependencies_transitive() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();

    // grandchild.cdm should have 2 total dependencies: child and base
    let all_deps = graph.get_all_dependencies(&files[2]);
    assert_eq!(all_deps.len(), 2);
    assert!(all_deps.contains(&files[0])); // base
    assert!(all_deps.contains(&files[1])); // child
}

#[test]
fn test_depends_on() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();

    // grandchild depends on base (transitively)
    assert!(graph.depends_on(&files[2], &files[0]));

    // grandchild depends on child (directly)
    assert!(graph.depends_on(&files[2], &files[1]));

    // base does not depend on child
    assert!(!graph.depends_on(&files[0], &files[1]));
}

#[test]
fn test_empty_graph() {
    let graph = DependencyGraph::build(&[]).unwrap();

    let deps = graph.get_all_dependents(Path::new("/nonexistent.cdm"));
    assert!(deps.is_empty());
}

#[test]
fn test_file_with_missing_extends() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create a file that extends a non-existent file
    let mut orphan = File::create(root.join("orphan.cdm")).unwrap();
    writeln!(orphan, "extends \"./missing.cdm\"").unwrap();
    writeln!(orphan, "Orphan {{}}").unwrap();

    let files = vec![root.join("orphan.cdm").canonicalize().unwrap()];

    // Should succeed but with no dependencies tracked
    let graph = DependencyGraph::build(&files).unwrap();
    let deps = graph.direct_dependencies(&files[0]);
    assert!(deps.is_empty());
}

#[test]
fn test_results_are_sorted() {
    let temp = create_test_project();
    let root = temp.path();

    let files = vec![
        root.join("base.cdm").canonicalize().unwrap(),
        root.join("src/child.cdm").canonicalize().unwrap(),
        root.join("src/grandchild.cdm").canonicalize().unwrap(),
        root.join("src/sibling.cdm").canonicalize().unwrap(),
    ];

    let graph = DependencyGraph::build(&files).unwrap();

    let dependents = graph.get_all_dependents(&files[0]);

    // Verify sorted
    let mut sorted = dependents.clone();
    sorted.sort();
    assert_eq!(dependents, sorted);
}
