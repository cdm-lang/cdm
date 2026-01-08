use super::*;

#[test]
fn test_workspace_update_and_dependencies() {
    let workspace = Workspace::new();

    let uri1 = Url::parse("file:///base.cdm").unwrap();
    let uri2 = Url::parse("file:///derived.cdm").unwrap();

    // Update base file (no dependencies)
    workspace.update_document(uri1.clone(), "User { name: string #1 } #10".to_string());

    // Update derived file (extends base.cdm)
    workspace.update_document(
        uri2.clone(),
        "extends ./base.cdm\n\nAdminUser extends User { role: string #1 } #20".to_string(),
    );

    // Check dependencies
    let deps = workspace.get_dependency_chain(&uri2);
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], uri1);

    // Check dependents
    let dependents = workspace.get_all_dependents(&uri1);
    assert_eq!(dependents.len(), 1);
    assert!(dependents.contains(&uri2));
}

#[test]
fn test_workspace_remove_document() {
    let workspace = Workspace::new();

    let uri1 = Url::parse("file:///base.cdm").unwrap();
    let uri2 = Url::parse("file:///derived.cdm").unwrap();

    workspace.update_document(uri1.clone(), "User { } #10".to_string());
    workspace.update_document(uri2.clone(), "extends ./base.cdm".to_string());

    // Remove the derived file
    workspace.remove_document(&uri2);

    // Should no longer have dependents
    let dependents = workspace.get_all_dependents(&uri1);
    assert_eq!(dependents.len(), 0);
}

#[test]
fn test_extract_extends_directives() {
    let text = r#"
extends ./base.cdm
extends ./mixins/timestamps.cdm

User extends BaseUser {
  name: string #1
} #10
"#;

    let extends = extract_extends_directives(text);
    assert_eq!(extends.len(), 2);
    assert!(extends.contains(&"./base.cdm".to_string()));
    assert!(extends.contains(&"./mixins/timestamps.cdm".to_string()));
}

#[test]
fn test_cached_text() {
    let workspace = Workspace::new();
    let uri = Url::parse("file:///test.cdm").unwrap();

    let text = "User { name: string #1 } #10";
    workspace.update_document(uri.clone(), text.to_string());

    let cached = workspace.get_cached_text(&uri);
    assert_eq!(cached, Some(text.to_string()));
}
