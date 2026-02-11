use std::collections::BTreeMap;

/// Helper for building TypeORM decorator strings
#[derive(Debug, Clone)]
pub struct DecoratorBuilder {
    name: String,
    args: Vec<String>,
    options: BTreeMap<String, String>,
}

impl DecoratorBuilder {
    /// Create a new decorator builder with the given decorator name
    pub fn new(name: &str) -> Self {
        DecoratorBuilder {
            name: name.to_string(),
            args: Vec::new(),
            options: BTreeMap::new(),
        }
    }

    /// Create an @Entity decorator
    pub fn entity() -> Self {
        Self::new("Entity")
    }

    /// Create a @Column decorator
    pub fn column() -> Self {
        Self::new("Column")
    }

    /// Create a @PrimaryColumn decorator
    pub fn primary_column() -> Self {
        Self::new("PrimaryColumn")
    }

    /// Create a @CreateDateColumn decorator
    pub fn create_date_column() -> Self {
        Self::new("CreateDateColumn")
    }

    /// Create an @UpdateDateColumn decorator
    pub fn update_date_column() -> Self {
        Self::new("UpdateDateColumn")
    }

    /// Create a @DeleteDateColumn decorator
    pub fn delete_date_column() -> Self {
        Self::new("DeleteDateColumn")
    }

    /// Create a @PrimaryGeneratedColumn decorator with generation strategy
    pub fn primary_generated_column(strategy: Option<&str>) -> Self {
        let mut builder = Self::new("PrimaryGeneratedColumn");
        if let Some(s) = strategy {
            builder.args.push(format!("\"{}\"", s));
        }
        builder
    }

    /// Create a @OneToOne decorator
    pub fn one_to_one(target: &str, inverse: Option<&str>) -> Self {
        let mut builder = Self::new("OneToOne");
        builder.args.push(format!("() => {}", target));
        if let Some(inv) = inverse {
            builder
                .args
                .push(format!("({}) => {}.{}", to_param_name(target), to_param_name(target), inv));
        }
        builder
    }

    /// Create a @OneToMany decorator
    pub fn one_to_many(target: &str, inverse: &str) -> Self {
        let mut builder = Self::new("OneToMany");
        builder.args.push(format!("() => {}", target));
        builder
            .args
            .push(format!("({}) => {}.{}", to_param_name(target), to_param_name(target), inverse));
        builder
    }

    /// Create a @ManyToOne decorator
    pub fn many_to_one(target: &str, inverse: Option<&str>) -> Self {
        let mut builder = Self::new("ManyToOne");
        builder.args.push(format!("() => {}", target));
        if let Some(inv) = inverse {
            builder
                .args
                .push(format!("({}) => {}.{}", to_param_name(target), to_param_name(target), inv));
        }
        builder
    }

    /// Create a @ManyToMany decorator
    pub fn many_to_many(target: &str, inverse: Option<&str>) -> Self {
        let mut builder = Self::new("ManyToMany");
        builder.args.push(format!("() => {}", target));
        if let Some(inv) = inverse {
            builder
                .args
                .push(format!("({}) => {}.{}", to_param_name(target), to_param_name(target), inv));
        }
        builder
    }

    /// Create a @JoinColumn decorator
    pub fn join_column() -> Self {
        Self::new("JoinColumn")
    }

    /// Create a @JoinTable decorator
    #[allow(dead_code)]
    pub fn join_table() -> Self {
        Self::new("JoinTable")
    }

    /// Create an @Index decorator
    pub fn index(fields: &[String]) -> Self {
        let mut builder = Self::new("Index");
        let fields_str = fields
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<_>>()
            .join(", ");
        builder.args.push(format!("[{}]", fields_str));
        builder
    }

    // Lifecycle hook decorators

    /// Create a @BeforeInsert decorator
    pub fn before_insert() -> Self {
        Self::new("BeforeInsert")
    }

    /// Create an @AfterInsert decorator
    pub fn after_insert() -> Self {
        Self::new("AfterInsert")
    }

    /// Create a @BeforeUpdate decorator
    pub fn before_update() -> Self {
        Self::new("BeforeUpdate")
    }

    /// Create an @AfterUpdate decorator
    pub fn after_update() -> Self {
        Self::new("AfterUpdate")
    }

    /// Create a @BeforeRemove decorator
    pub fn before_remove() -> Self {
        Self::new("BeforeRemove")
    }

    /// Create an @AfterRemove decorator
    pub fn after_remove() -> Self {
        Self::new("AfterRemove")
    }

    /// Create an @AfterLoad decorator
    pub fn after_load() -> Self {
        Self::new("AfterLoad")
    }

    /// Create a @BeforeSoftRemove decorator
    pub fn before_soft_remove() -> Self {
        Self::new("BeforeSoftRemove")
    }

    /// Create an @AfterSoftRemove decorator
    pub fn after_soft_remove() -> Self {
        Self::new("AfterSoftRemove")
    }

    /// Create an @AfterRecover decorator
    pub fn after_recover() -> Self {
        Self::new("AfterRecover")
    }

    /// Add a positional argument
    #[allow(dead_code)]
    pub fn arg(mut self, value: String) -> Self {
        self.args.push(value);
        self
    }

    /// Add a string option (will be quoted)
    pub fn string_option(mut self, key: &str, value: &str) -> Self {
        self.options.insert(key.to_string(), format!("\"{}\"", value));
        self
    }

    /// Add a boolean option
    pub fn bool_option(mut self, key: &str, value: bool) -> Self {
        self.options
            .insert(key.to_string(), if value { "true" } else { "false" }.to_string());
        self
    }

    /// Add a number option
    pub fn number_option(mut self, key: &str, value: i64) -> Self {
        self.options.insert(key.to_string(), value.to_string());
        self
    }

    /// Add a raw option (unquoted - for expressions, arrays, etc.)
    #[allow(dead_code)]
    pub fn raw_option(mut self, key: &str, value: &str) -> Self {
        self.options.insert(key.to_string(), value.to_string());
        self
    }

    /// Add options object for relation decorators
    pub fn with_relation_options(
        mut self,
        cascade: Option<bool>,
        eager: Option<bool>,
        lazy: Option<bool>,
        nullable: Option<bool>,
        on_delete: Option<&str>,
        on_update: Option<&str>,
    ) -> Self {
        if let Some(c) = cascade {
            self.options.insert("cascade".to_string(), c.to_string());
        }
        if let Some(e) = eager {
            self.options.insert("eager".to_string(), e.to_string());
        }
        if let Some(l) = lazy {
            self.options.insert("lazy".to_string(), l.to_string());
        }
        if let Some(n) = nullable {
            self.options.insert("nullable".to_string(), n.to_string());
        }
        if let Some(od) = on_delete {
            self.options
                .insert("onDelete".to_string(), format!("\"{}\"", od));
        }
        if let Some(ou) = on_update {
            self.options
                .insert("onUpdate".to_string(), format!("\"{}\"", ou));
        }
        self
    }

    /// Build the decorator string
    pub fn build(&self) -> String {
        let mut result = format!("@{}", self.name);

        // Combine args and options
        let has_args = !self.args.is_empty();
        let has_options = !self.options.is_empty();

        if !has_args && !has_options {
            result.push_str("()");
            return result;
        }

        result.push('(');

        // Add positional args
        if has_args {
            result.push_str(&self.args.join(", "));
        }

        // Add options object
        if has_options {
            if has_args {
                result.push_str(", ");
            }
            result.push_str("{ ");
            let opts: Vec<String> = self
                .options
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect();
            result.push_str(&opts.join(", "));
            result.push_str(" }");
        }

        result.push(')');
        result
    }
}

/// Convert a model name to a parameter name (first letter lowercase)
fn to_param_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

/// Build a @JoinColumn decorator with options
pub fn build_join_column(name: Option<&str>, referenced_column: Option<&str>) -> String {
    let mut builder = DecoratorBuilder::join_column();

    if let Some(n) = name {
        builder = builder.string_option("name", n);
    }
    if let Some(rc) = referenced_column {
        builder = builder.string_option("referencedColumnName", rc);
    }

    builder.build()
}

/// Build a @JoinTable decorator with options
pub fn build_join_table(
    name: &str,
    join_column_name: Option<&str>,
    join_column_ref: Option<&str>,
    inverse_column_name: Option<&str>,
    inverse_column_ref: Option<&str>,
) -> String {
    let mut parts = vec![format!("name: \"{}\"", name)];

    // Build joinColumn if specified
    if join_column_name.is_some() || join_column_ref.is_some() {
        let mut jc_parts = Vec::new();
        if let Some(n) = join_column_name {
            jc_parts.push(format!("name: \"{}\"", n));
        }
        if let Some(r) = join_column_ref {
            jc_parts.push(format!("referencedColumnName: \"{}\"", r));
        }
        parts.push(format!("joinColumn: {{ {} }}", jc_parts.join(", ")));
    }

    // Build inverseJoinColumn if specified
    if inverse_column_name.is_some() || inverse_column_ref.is_some() {
        let mut ijc_parts = Vec::new();
        if let Some(n) = inverse_column_name {
            ijc_parts.push(format!("name: \"{}\"", n));
        }
        if let Some(r) = inverse_column_ref {
            ijc_parts.push(format!("referencedColumnName: \"{}\"", r));
        }
        parts.push(format!("inverseJoinColumn: {{ {} }}", ijc_parts.join(", ")));
    }

    format!("@JoinTable({{ {} }})", parts.join(", "))
}

#[cfg(test)]
#[path = "decorator_builder/decorator_builder_tests.rs"]
mod decorator_builder_tests;
