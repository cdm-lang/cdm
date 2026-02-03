//! CDM SQL plugin for generating SQL schemas and migrations.

mod build;
mod migrate;
mod type_mapper;
mod utils;
mod validate;

use cdm_plugin_interface::schema_from_file;

pub use build::build;
pub use migrate::migrate;
pub use validate::validate_config;

// Embed schema.cdm and export via WASM (required)
schema_from_file!("../schema.cdm");

// Export WASM functions using the FFI helpers from cdm-plugin-interface
cdm_plugin_interface::export_validate_config!(validate_config);
cdm_plugin_interface::export_build!(build);
cdm_plugin_interface::export_migrate!(migrate);

// Export standard memory management functions
pub use cdm_plugin_interface::ffi::{_alloc, _dealloc};
