//! Main configuration initialization.

use crate::logging::with_config_context;
use ngx::ffi::ngx_conf_t;

/// # Safety
///
/// Caller must pass valid NGINX pointers.
pub unsafe extern "C" fn init_main_config(
    cf: *mut ngx_conf_t,
    _conf: *mut core::ffi::c_void,
) -> *mut core::ffi::c_char {
    with_config_context(cf, core::ptr::null_mut)
}
