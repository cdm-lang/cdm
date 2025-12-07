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
    slice::from_raw_parts(ptr, len).to_vec()
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
    let level_bytes = read_bytes(level_ptr, level_len);
    let config_bytes = read_bytes(config_ptr, config_len);

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

/// Helper for implementing _generate WASM export
///
/// # Safety
/// This function is unsafe because it works with raw pointers from WASM.
pub unsafe fn ffi_generate<F>(
    schema_ptr: *const u8,
    schema_len: usize,
    config_ptr: *const u8,
    config_len: usize,
    generate_fn: F,
) -> *mut u8
where
    F: Fn(Schema, JSON, &Utils) -> Vec<OutputFile>,
{
    // Read inputs from WASM memory
    let schema_bytes = read_bytes(schema_ptr, schema_len);
    let config_bytes = read_bytes(config_ptr, config_len);

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

    // Call the actual generate function
    let utils = Utils;
    let files = generate_fn(schema, config, &utils);

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
    let schema_bytes = read_bytes(schema_ptr, schema_len);
    let deltas_bytes = read_bytes(deltas_ptr, deltas_len);
    let config_bytes = read_bytes(config_ptr, config_len);

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
#[no_mangle]
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
#[no_mangle]
pub unsafe extern "C" fn _dealloc(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

/// Macro to export validate_config function with proper FFI wrapper
#[macro_export]
macro_rules! export_validate_config {
    ($func:expr) => {
        #[no_mangle]
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

/// Macro to export generate function with proper FFI wrapper
#[macro_export]
macro_rules! export_generate {
    ($func:expr) => {
        #[no_mangle]
        pub extern "C" fn _generate(
            schema_ptr: *const u8,
            schema_len: usize,
            config_ptr: *const u8,
            config_len: usize,
        ) -> *mut u8 {
            unsafe {
                $crate::ffi::ffi_generate(
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
        #[no_mangle]
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
