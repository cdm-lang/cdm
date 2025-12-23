mod validate;
mod build;

use cdm_plugin_api::schema_from_file;

pub use validate::validate_config;
pub use build::build;

// Embed schema.cdm and export via WASM (required)
schema_from_file!("../schema.cdm");

// Export WASM functions using the FFI helpers from cdm-plugin-api
// Note: validate_config and build are both optional.
// This plugin implements both to demonstrate their usage.
cdm_plugin_api::export_validate_config!(validate_config);
cdm_plugin_api::export_build!(build);

// Export standard memory management functions
pub use cdm_plugin_api::ffi::{_alloc, _dealloc};
