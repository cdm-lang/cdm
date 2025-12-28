//! FFI helpers for WASM plugins
//!
//! This module provides utilities for plugin authors to implement the WASM FFI layer
//! without having to deal with raw pointers and memory management.

use crate::*;
use std::slice;

/// Read bytes from WASM memory using a pointer and length
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure the pointer and length are valid.
unsafe fn read_bytes(ptr: *const u8, len: usize) -> Vec<u8> {
    if ptr.is_null() || len == 0 {
        return Vec::new();
    }
    unsafe { slice::from_raw_parts(ptr, len).to_vec() }
}

/// Allocate memory and write bytes, returning a pointer with length prefix
///
/// Format: [4-byte length (little-endian)][data bytes]
fn write_result(data: &[u8]) -> *mut u8 {
    let total_len = 4 + data.len();
    let mut buffer = Vec::with_capacity(total_len);

    // Write length prefix (4 bytes, little-endian)
    let len_bytes = (data.len() as u32).to_le_bytes();
    buffer.extend_from_slice(&len_bytes);

    // Write actual data
    buffer.extend_from_slice(data);

    // Convert to raw pointer and forget the Vec so it's not deallocated
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Helper for implementing _schema WASM export
///
/// This function is safe because it doesn't work with raw pointers (other than the return value).
pub fn ffi_schema<F>(schema_fn: F) -> *mut u8
where
    F: Fn() -> String,
{
    // Call the actual schema function
    let schema_content = schema_fn();

    // Write result to WASM memory and return pointer
    write_result(schema_content.as_bytes())
}

/// Helper for implementing _validate_config WASM export
///
/// # Safety
/// This function is unsafe because it works with raw pointers from WASM.
pub unsafe fn ffi_validate_config<F>(
    level_ptr: *const u8,
    level_len: usize,
    config_ptr: *const u8,
    config_len: usize,
    validate_fn: F,
) -> *mut u8
where
    F: Fn(ConfigLevel, JSON, &Utils) -> Vec<ValidationError>,
{
    // Read inputs from WASM memory
    let level_bytes = unsafe { read_bytes(level_ptr, level_len) };
    let config_bytes = unsafe { read_bytes(config_ptr, config_len) };

    // Deserialize inputs
    let level: ConfigLevel = match serde_json::from_slice(&level_bytes) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to deserialize ConfigLevel: {}", e);
            return write_result(b"[]");
        }
    };

    let config: JSON = match serde_json::from_slice(&config_bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to deserialize config: {}", e);
            return write_result(b"[]");
        }
    };

    // Call the actual validation function
    let utils = Utils;
    let errors = validate_fn(level, config, &utils);

    // Serialize the result
    let result_json = match serde_json::to_vec(&errors) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to serialize validation errors: {}", e);
            return write_result(b"[]");
        }
    };

    // Write result to WASM memory and return pointer
    write_result(&result_json)
}

/// Helper for implementing _build WASM export
///
/// # Safety
/// This function is unsafe because it works with raw pointers from WASM.
pub unsafe fn ffi_build<F>(
    schema_ptr: *const u8,
    schema_len: usize,
    config_ptr: *const u8,
    config_len: usize,
    build_fn: F,
) -> *mut u8
where
    F: Fn(Schema, JSON, &Utils) -> Vec<OutputFile>,
{
    // Read inputs from WASM memory
    let schema_bytes = unsafe { read_bytes(schema_ptr, schema_len) };
    let config_bytes = unsafe { read_bytes(config_ptr, config_len) };

    // Deserialize inputs
    let schema: Schema = match serde_json::from_slice(&schema_bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to deserialize Schema: {}", e);
            return write_result(b"[]");
        }
    };

    let config: JSON = match serde_json::from_slice(&config_bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to deserialize config: {}", e);
            return write_result(b"[]");
        }
    };

    // Call the actual build function
    let utils = Utils;
    let files = build_fn(schema, config, &utils);

    // Serialize the result
    let result_json = match serde_json::to_vec(&files) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to serialize output files: {}", e);
            return write_result(b"[]");
        }
    };

    // Write result to WASM memory and return pointer
    write_result(&result_json)
}

/// Helper for implementing _migrate WASM export
///
/// # Safety
/// This function is unsafe because it works with raw pointers from WASM.
pub unsafe fn ffi_migrate<F>(
    schema_ptr: *const u8,
    schema_len: usize,
    deltas_ptr: *const u8,
    deltas_len: usize,
    config_ptr: *const u8,
    config_len: usize,
    migrate_fn: F,
) -> *mut u8
where
    F: Fn(Schema, Vec<Delta>, JSON, &Utils) -> Vec<OutputFile>,
{
    // Read inputs from WASM memory
    let schema_bytes = unsafe { read_bytes(schema_ptr, schema_len) };
    let deltas_bytes = unsafe { read_bytes(deltas_ptr, deltas_len) };
    let config_bytes = unsafe { read_bytes(config_ptr, config_len) };

    // Deserialize inputs
    let schema: Schema = match serde_json::from_slice(&schema_bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to deserialize Schema: {}", e);
            return write_result(b"[]");
        }
    };

    let deltas: Vec<Delta> = match serde_json::from_slice(&deltas_bytes) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to deserialize deltas: {}", e);
            return write_result(b"[]");
        }
    };

    let config: JSON = match serde_json::from_slice(&config_bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to deserialize config: {}", e);
            return write_result(b"[]");
        }
    };

    // Call the actual migrate function
    let utils = Utils;
    let files = migrate_fn(schema, deltas, config, &utils);

    // Serialize the result
    let result_json = match serde_json::to_vec(&files) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to serialize migration files: {}", e);
            return write_result(b"[]");
        }
    };

    // Write result to WASM memory and return pointer
    write_result(&result_json)
}

/// Standard WASM memory allocation function
///
/// This should be exported as `_alloc` by all plugins.
#[unsafe(no_mangle)]
pub extern "C" fn _alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Standard WASM memory deallocation function
///
/// This should be exported as `_dealloc` by all plugins.
///
/// # Safety
/// This function is unsafe because it deallocates raw pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _dealloc(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        let _ = unsafe { Vec::from_raw_parts(ptr, 0, size) };
    }
}

/// Macro to export schema function with proper FFI wrapper
#[macro_export]
macro_rules! export_schema {
    ($func:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn _schema() -> *mut u8 {
            $crate::ffi::ffi_schema($func)
        }
    };
}

/// Macro to export validate_config function with proper FFI wrapper
#[macro_export]
macro_rules! export_validate_config {
    ($func:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn _validate_config(
            level_ptr: *const u8,
            level_len: usize,
            config_ptr: *const u8,
            config_len: usize,
        ) -> *mut u8 {
            unsafe {
                $crate::ffi::ffi_validate_config(
                    level_ptr,
                    level_len,
                    config_ptr,
                    config_len,
                    $func,
                )
            }
        }
    };
}

/// Macro to export build function with proper FFI wrapper
#[macro_export]
macro_rules! export_build {
    ($func:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn _build(
            schema_ptr: *const u8,
            schema_len: usize,
            config_ptr: *const u8,
            config_len: usize,
        ) -> *mut u8 {
            unsafe {
                $crate::ffi::ffi_build(
                    schema_ptr,
                    schema_len,
                    config_ptr,
                    config_len,
                    $func,
                )
            }
        }
    };
}

/// Macro to export migrate function with proper FFI wrapper
#[macro_export]
macro_rules! export_migrate {
    ($func:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn _migrate(
            schema_ptr: *const u8,
            schema_len: usize,
            deltas_ptr: *const u8,
            deltas_len: usize,
            config_ptr: *const u8,
            config_len: usize,
        ) -> *mut u8 {
            unsafe {
                $crate::ffi::ffi_migrate(
                    schema_ptr,
                    schema_len,
                    deltas_ptr,
                    deltas_len,
                    config_ptr,
                    config_len,
                    $func,
                )
            }
        }
    };
}

#[cfg(test)]
mod tests {
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
}
