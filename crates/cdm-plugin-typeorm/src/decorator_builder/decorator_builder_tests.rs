use super::*;

#[test]
fn test_entity_decorator_empty() {
    let decorator = DecoratorBuilder::entity().build();
    assert_eq!(decorator, "@Entity()");
}

#[test]
fn test_entity_decorator_with_name() {
    let decorator = DecoratorBuilder::entity()
        .string_option("name", "users")
        .build();
    assert_eq!(decorator, "@Entity({ name: \"users\" })");
}

#[test]
fn test_column_decorator_empty() {
    let decorator = DecoratorBuilder::column().build();
    assert_eq!(decorator, "@Column()");
}

#[test]
fn test_column_decorator_with_options() {
    let decorator = DecoratorBuilder::column()
        .string_option("type", "varchar")
        .number_option("length", 255)
        .bool_option("nullable", true)
        .build();
    // BTreeMap sorts keys alphabetically
    assert!(decorator.contains("length: 255"));
    assert!(decorator.contains("nullable: true"));
    assert!(decorator.contains("type: \"varchar\""));
}

#[test]
fn test_primary_generated_column_uuid() {
    let decorator = DecoratorBuilder::primary_generated_column(Some("uuid")).build();
    assert_eq!(decorator, "@PrimaryGeneratedColumn(\"uuid\")");
}

#[test]
fn test_primary_generated_column_increment() {
    let decorator = DecoratorBuilder::primary_generated_column(Some("increment")).build();
    assert_eq!(decorator, "@PrimaryGeneratedColumn(\"increment\")");
}

#[test]
fn test_primary_generated_column_default() {
    let decorator = DecoratorBuilder::primary_generated_column(None).build();
    assert_eq!(decorator, "@PrimaryGeneratedColumn()");
}

#[test]
fn test_one_to_many_decorator() {
    let decorator = DecoratorBuilder::one_to_many("Post", "author").build();
    assert_eq!(decorator, "@OneToMany(() => Post, (post) => post.author)");
}

#[test]
fn test_many_to_one_decorator() {
    let decorator = DecoratorBuilder::many_to_one("User", Some("posts")).build();
    assert_eq!(decorator, "@ManyToOne(() => User, (user) => user.posts)");
}

#[test]
fn test_many_to_one_decorator_no_inverse() {
    let decorator = DecoratorBuilder::many_to_one("User", None).build();
    assert_eq!(decorator, "@ManyToOne(() => User)");
}

#[test]
fn test_many_to_many_decorator() {
    let decorator = DecoratorBuilder::many_to_many("Tag", Some("posts")).build();
    assert_eq!(decorator, "@ManyToMany(() => Tag, (tag) => tag.posts)");
}

#[test]
fn test_index_decorator() {
    let decorator = DecoratorBuilder::index(&["email".to_string()]).build();
    assert_eq!(decorator, "@Index([\"email\"])");
}

#[test]
fn test_index_decorator_multi_column() {
    let decorator = DecoratorBuilder::index(&["firstName".to_string(), "lastName".to_string()]).build();
    assert_eq!(decorator, "@Index([\"firstName\", \"lastName\"])");
}

#[test]
fn test_index_decorator_unique() {
    let decorator = DecoratorBuilder::index(&["email".to_string()])
        .bool_option("unique", true)
        .build();
    assert!(decorator.contains("unique: true"));
}

#[test]
fn test_build_join_column() {
    let jc = build_join_column(Some("user_id"), None);
    assert_eq!(jc, "@JoinColumn({ name: \"user_id\" })");
}

#[test]
fn test_build_join_column_with_reference() {
    let jc = build_join_column(Some("user_id"), Some("uuid"));
    assert!(jc.contains("name: \"user_id\""));
    assert!(jc.contains("referencedColumnName: \"uuid\""));
}

#[test]
fn test_build_join_table() {
    let jt = build_join_table("post_tags", None, None, None, None);
    assert_eq!(jt, "@JoinTable({ name: \"post_tags\" })");
}

#[test]
fn test_build_join_table_with_columns() {
    let jt = build_join_table(
        "post_tags",
        Some("post_id"),
        None,
        Some("tag_id"),
        None,
    );
    assert!(jt.contains("name: \"post_tags\""));
    assert!(jt.contains("joinColumn: { name: \"post_id\" }"));
    assert!(jt.contains("inverseJoinColumn: { name: \"tag_id\" }"));
}

#[test]
fn test_relation_options() {
    let decorator = DecoratorBuilder::many_to_one("User", Some("posts"))
        .with_relation_options(
            Some(true),  // cascade
            None,        // eager
            None,        // lazy
            Some(false), // nullable
            Some("CASCADE"),
            None,
        )
        .build();

    assert!(decorator.contains("cascade: true"));
    assert!(decorator.contains("nullable: false"));
    assert!(decorator.contains("onDelete: \"CASCADE\""));
}

// Lifecycle hook decorator tests

#[test]
fn test_before_insert_decorator() {
    let decorator = DecoratorBuilder::before_insert().build();
    assert_eq!(decorator, "@BeforeInsert()");
}

#[test]
fn test_after_insert_decorator() {
    let decorator = DecoratorBuilder::after_insert().build();
    assert_eq!(decorator, "@AfterInsert()");
}

#[test]
fn test_before_update_decorator() {
    let decorator = DecoratorBuilder::before_update().build();
    assert_eq!(decorator, "@BeforeUpdate()");
}

#[test]
fn test_after_update_decorator() {
    let decorator = DecoratorBuilder::after_update().build();
    assert_eq!(decorator, "@AfterUpdate()");
}

#[test]
fn test_before_remove_decorator() {
    let decorator = DecoratorBuilder::before_remove().build();
    assert_eq!(decorator, "@BeforeRemove()");
}

#[test]
fn test_after_remove_decorator() {
    let decorator = DecoratorBuilder::after_remove().build();
    assert_eq!(decorator, "@AfterRemove()");
}

#[test]
fn test_after_load_decorator() {
    let decorator = DecoratorBuilder::after_load().build();
    assert_eq!(decorator, "@AfterLoad()");
}

#[test]
fn test_before_soft_remove_decorator() {
    let decorator = DecoratorBuilder::before_soft_remove().build();
    assert_eq!(decorator, "@BeforeSoftRemove()");
}

#[test]
fn test_after_soft_remove_decorator() {
    let decorator = DecoratorBuilder::after_soft_remove().build();
    assert_eq!(decorator, "@AfterSoftRemove()");
}

#[test]
fn test_after_recover_decorator() {
    let decorator = DecoratorBuilder::after_recover().build();
    assert_eq!(decorator, "@AfterRecover()");
}
