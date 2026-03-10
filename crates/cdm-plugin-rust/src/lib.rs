mod build;
mod type_mapper;
mod validate;

use cdm_plugin_interface::schema_from_file;

pub use build::build;
pub use validate::validate_config;

schema_from_file!("../schema.cdm");

cdm_plugin_interface::export_validate_config!(validate_config);
cdm_plugin_interface::export_build!(build);

pub use cdm_plugin_interface::ffi::{_alloc, _dealloc};
