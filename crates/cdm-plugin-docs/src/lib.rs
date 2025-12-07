mod validate;
mod generate;

pub use validate::validate_config;
pub use generate::generate;

// Export WASM functions using the FFI helpers from cdm-plugin-api
cdm_plugin_api::export_validate_config!(validate_config);
cdm_plugin_api::export_generate!(generate);

// Export standard memory management functions
pub use cdm_plugin_api::ffi::{_alloc, _dealloc};
