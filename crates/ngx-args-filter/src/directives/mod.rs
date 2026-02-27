//! Directive registration.

#![allow(static_mut_refs)]

pub mod args_filter;
pub mod args_filter_nested;

use ngx::ffi::ngx_command_t;

/// Shared nginx command-array terminator sentinel.
pub const NGX_EMPTY_COMMAND: ngx_command_t = ngx_command_t {
    name: ngx::ffi::ngx_str_t {
        len: 0,
        data: core::ptr::null_mut(),
    },
    type_: 0,
    set: None,
    conf: 0,
    offset: 0,
    post: core::ptr::null_mut(),
};

#[unsafe(no_mangle)]
pub static mut DIRECTIVES: [ngx_command_t; 2] = [
    unsafe { args_filter::ARGS_FILTER_COMMAND },
    NGX_EMPTY_COMMAND,
];
