//! `ngx-args-filter-module` nginx module.
//!
//! The module exposes `args_filter` directives to construct filtered query-string
//! variables at request time.

#![allow(improper_ctypes)]
#![allow(static_mut_refs)]

mod conf_ext;
mod config;
mod directives;
mod logging;
mod nginx_str;
mod status;
mod version;

use config::MainConf;
use ngx::{
    ffi::{NGX_HTTP_MODULE, ngx_module_t},
    http::{HttpModule, HttpModuleMainConf},
};

pub struct NgxArgsFilterModule;

unsafe impl HttpModuleMainConf for NgxArgsFilterModule {
    type MainConf = MainConf;
}

impl HttpModule for NgxArgsFilterModule {
    fn module() -> &'static ngx_module_t {
        unsafe { &*core::ptr::addr_of!(ngx_http_ngx_args_filter_module) }
    }
}

impl NgxArgsFilterModule {
    /// # Safety
    ///
    /// Caller must pass valid pointers from NGINX config phase.
    pub unsafe extern "C" fn init_main_conf(
        cf: *mut ngx::ffi::ngx_conf_t,
        conf: *mut core::ffi::c_void,
    ) -> *mut core::ffi::c_char {
        use crate::config::init::init_main_config;
        unsafe { init_main_config(cf, conf) }
    }
}

ngx::ngx_modules!(ngx_http_ngx_args_filter_module);

#[used]
#[allow(non_upper_case_globals)]
pub static mut ngx_http_ngx_args_filter_module: ngx_module_t = ngx_module_t {
    ctx: std::ptr::addr_of!(ngx_http_ngx_args_filter_module_ctx) as _,
    commands: { &raw const directives::DIRECTIVES as *mut _ },
    type_: NGX_HTTP_MODULE as _,
    version: version::NGX_VERSION_NUMBER,
    ..ngx_module_t::default()
};

#[unsafe(no_mangle)]
pub static ngx_http_ngx_args_filter_module_ctx: ngx::ffi::ngx_http_module_t =
    ngx::ffi::ngx_http_module_t {
        preconfiguration: None,
        postconfiguration: None,
        create_main_conf: Some(NgxArgsFilterModule::create_main_conf),
        init_main_conf: Some(NgxArgsFilterModule::init_main_conf),
        create_srv_conf: None,
        merge_srv_conf: None,
        create_loc_conf: None,
        merge_loc_conf: None,
    };
