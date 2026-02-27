//! Typed wrappers for common NGINX status codes.

use ngx::ffi::{NGX_DECLINED, NGX_ERROR, NGX_OK, ngx_int_t};

#[allow(clippy::cast_possible_wrap)]
impl NgxStatus {
    pub const OK: ngx_int_t = NGX_OK as ngx_int_t;
    pub const ERROR: ngx_int_t = NGX_ERROR as ngx_int_t;
    pub const DECLINED: ngx_int_t = NGX_DECLINED as ngx_int_t;
}

pub struct NgxStatus;
