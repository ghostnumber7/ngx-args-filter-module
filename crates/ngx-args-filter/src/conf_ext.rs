//! Extension traits for nginx-sys types
//! Based on nginx-acme pattern for safe, readable access

use ngx::core::Pool;
use ngx::ffi::{ngx_conf_t, ngx_str_t};

pub trait NgxConfExt {
    fn args(&self) -> &[ngx_str_t];
    fn pool(&self) -> Pool;
}

impl NgxConfExt for ngx_conf_t {
    fn args(&self) -> &[ngx_str_t] {
        // SAFETY: we know that cf.args is an array of ngx_str_t
        unsafe { self.args.as_ref().map(|x| x.as_slice()).unwrap_or_default() }
    }

    fn pool(&self) -> Pool {
        // SAFETY: `cf` always has a valid pool
        unsafe { Pool::from_ngx_pool(self.pool) }
    }
}
