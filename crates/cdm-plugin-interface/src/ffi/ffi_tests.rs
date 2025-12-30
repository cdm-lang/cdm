use super::*;

#[test]
fn test_ffi_schema() {
    let schema_fn = || "test schema content".to_string();
    let result_ptr = ffi_schema(schema_fn);

    // Verify pointer is not null
    assert!(!result_ptr.is_null());

    // Read the result
    unsafe {
        // First 4 bytes are length
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        // Read the actual data
        let data = std::slice::from_raw_parts(result_ptr.add(4), len);
        let result = std::str::from_utf8(data).unwrap();

        assert_eq!(result, "test schema content");

        // Clean up
        _dealloc(result_ptr, 4 + len);
    }
}

#[test]
fn test_ffi_schema_empty() {
    let schema_fn = || "".to_string();
    let result_ptr = ffi_schema(schema_fn);

    assert!(!result_ptr.is_null());

    unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        assert_eq!(len, 0);

        _dealloc(result_ptr, 4);
    }
}

#[test]
fn test_ffi_validate_config() {
    let validate_fn = |_level: ConfigLevel, _config: JSON, _utils: &Utils| {
        vec![ValidationError {
            message: "test error".to_string(),
            path: vec![],
            severity: Severity::Error,
        }]
    };

    // Prepare inputs
    let level = ConfigLevel::Global;
    let level_json = serde_json::to_vec(&level).unwrap();

    let config = serde_json::json!({"key": "value"});
    let config_json = serde_json::to_vec(&config).unwrap();

    let result_ptr = unsafe {
        ffi_validate_config(
            level_json.as_ptr(),
            level_json.len(),
            config_json.as_ptr(),
            config_json.len(),
            validate_fn,
        )
    };

    assert!(!result_ptr.is_null());

    unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        let data = std::slice::from_raw_parts(result_ptr.add(4), len);
        let errors: Vec<ValidationError> = serde_json::from_slice(data).unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "test error");

        _dealloc(result_ptr, 4 + len);
    }
}

#[test]
fn test_ffi_validate_config_invalid_level() {
    let validate_fn = |_level: ConfigLevel, _config: JSON, _utils: &Utils| vec![];

    let invalid_json = b"invalid json";
    let config_json = b"{}";

    let result_ptr = unsafe {
        ffi_validate_config(
            invalid_json.as_ptr(),
            invalid_json.len(),
            config_json.as_ptr(),
            config_json.len(),
            validate_fn,
        )
    };

    assert!(!result_ptr.is_null());

    unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        let data = std::slice::from_raw_parts(result_ptr.add(4), len);
        let result = std::str::from_utf8(data).unwrap();
        assert_eq!(result, "[]");

        _dealloc(result_ptr, 4 + len);
    }
}

#[test]
fn test_ffi_validate_config_invalid_config() {
    let validate_fn = |_level: ConfigLevel, _config: JSON, _utils: &Utils| vec![];

    let level = ConfigLevel::Global;
    let level_json = serde_json::to_vec(&level).unwrap();
    let invalid_json = b"invalid json";

    let result_ptr = unsafe {
        ffi_validate_config(
            level_json.as_ptr(),
            level_json.len(),
            invalid_json.as_ptr(),
            invalid_json.len(),
            validate_fn,
        )
    };

    assert!(!result_ptr.is_null());

    unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        let data = std::slice::from_raw_parts(result_ptr.add(4), len);
        let result = std::str::from_utf8(data).unwrap();
        assert_eq!(result, "[]");

        _dealloc(result_ptr, 4 + len);
    }
}

#[test]
fn test_ffi_build() {
    use std::collections::HashMap;

    let build_fn = |_schema: Schema, _config: JSON, _utils: &Utils| {
        vec![OutputFile {
            path: "test.txt".to_string(),
            content: "test content".to_string(),
        }]
    };

    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };
    let schema_json = serde_json::to_vec(&schema).unwrap();

    let config = serde_json::json!({});
    let config_json = serde_json::to_vec(&config).unwrap();

    let result_ptr = unsafe {
        ffi_build(
            schema_json.as_ptr(),
            schema_json.len(),
            config_json.as_ptr(),
            config_json.len(),
            build_fn,
        )
    };

    assert!(!result_ptr.is_null());

    unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        let data = std::slice::from_raw_parts(result_ptr.add(4), len);
        let files: Vec<OutputFile> = serde_json::from_slice(data).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert_eq!(files[0].content, "test content");

        _dealloc(result_ptr, 4 + len);
    }
}

#[test]
fn test_ffi_build_invalid_schema() {
    let build_fn = |_schema: Schema, _config: JSON, _utils: &Utils| vec![];

    let invalid_json = b"invalid json";
    let config_json = b"{}";

    let result_ptr = unsafe {
        ffi_build(
            invalid_json.as_ptr(),
            invalid_json.len(),
            config_json.as_ptr(),
            config_json.len(),
            build_fn,
        )
    };

    assert!(!result_ptr.is_null());

    unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        let data = std::slice::from_raw_parts(result_ptr.add(4), len);
        let result = std::str::from_utf8(data).unwrap();
        assert_eq!(result, "[]");

        _dealloc(result_ptr, 4 + len);
    }
}

#[test]
fn test_ffi_migrate() {
    use std::collections::HashMap;

    let migrate_fn = |_schema: Schema, _deltas: Vec<Delta>, _config: JSON, _utils: &Utils| {
        vec![OutputFile {
            path: "migration.sql".to_string(),
            content: "CREATE TABLE test;".to_string(),
        }]
    };

    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };
    let schema_json = serde_json::to_vec(&schema).unwrap();

    let deltas: Vec<Delta> = vec![];
    let deltas_json = serde_json::to_vec(&deltas).unwrap();

    let config = serde_json::json!({});
    let config_json = serde_json::to_vec(&config).unwrap();

    let result_ptr = unsafe {
        ffi_migrate(
            schema_json.as_ptr(),
            schema_json.len(),
            deltas_json.as_ptr(),
            deltas_json.len(),
            config_json.as_ptr(),
            config_json.len(),
            migrate_fn,
        )
    };

    assert!(!result_ptr.is_null());

    unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        let data = std::slice::from_raw_parts(result_ptr.add(4), len);
        let files: Vec<OutputFile> = serde_json::from_slice(data).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "migration.sql");
        assert_eq!(files[0].content, "CREATE TABLE test;");

        _dealloc(result_ptr, 4 + len);
    }
}

#[test]
fn test_ffi_migrate_invalid_deltas() {
    use std::collections::HashMap;

    let migrate_fn = |_schema: Schema, _deltas: Vec<Delta>, _config: JSON, _utils: &Utils| vec![];

    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };
    let schema_json = serde_json::to_vec(&schema).unwrap();

    let invalid_json = b"invalid json";
    let config_json = b"{}";

    let result_ptr = unsafe {
        ffi_migrate(
            schema_json.as_ptr(),
            schema_json.len(),
            invalid_json.as_ptr(),
            invalid_json.len(),
            config_json.as_ptr(),
            config_json.len(),
            migrate_fn,
        )
    };

    assert!(!result_ptr.is_null());

    unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        let data = std::slice::from_raw_parts(result_ptr.add(4), len);
        let result = std::str::from_utf8(data).unwrap();
        assert_eq!(result, "[]");

        _dealloc(result_ptr, 4 + len);
    }
}

#[test]
fn test_alloc_dealloc() {
    unsafe {
        let ptr = _alloc(100);
        assert!(!ptr.is_null());
        _dealloc(ptr, 100);
    }
}

#[test]
fn test_alloc_zero_size() {
    unsafe {
        let ptr = _alloc(0);
        // Even zero-size allocation should return a valid pointer
        _dealloc(ptr, 0);
    }
}

#[test]
fn test_dealloc_null() {
    unsafe {
        // Should not panic
        _dealloc(std::ptr::null_mut(), 0);
    }
}

#[test]
fn test_read_bytes_null() {
    unsafe {
        let result = read_bytes(std::ptr::null(), 10);
        assert_eq!(result.len(), 0);
    }
}

#[test]
fn test_read_bytes_zero_length() {
    let data = b"test";
    unsafe {
        let result = read_bytes(data.as_ptr(), 0);
        assert_eq!(result.len(), 0);
    }
}

#[test]
fn test_read_bytes_valid() {
    let data = b"test data";
    unsafe {
        let result = read_bytes(data.as_ptr(), data.len());
        assert_eq!(result, data);
    }
}
