mod build;
mod type_mapper;
mod validate;

use cdm_plugin_api::schema_from_file;

pub use build::build;
pub use validate::validate_config;

// Embed schema.cdm and export via WASM (required)
schema_from_file!("../schema.cdm");

// Export WASM functions using the FFI helpers from cdm-plugin-api
cdm_plugin_api::export_validate_config!(validate_config);
cdm_plugin_api::export_build!(build);

// Export standard memory management functions
pub use cdm_plugin_api::ffi::{_alloc, _dealloc};
