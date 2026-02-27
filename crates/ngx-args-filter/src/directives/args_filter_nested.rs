//! Nested directives for `args_filter {}` blocks.
//!
//! Supported directives: `initial`, `include`, `exclude`, and `volatile`.

#![allow(static_mut_refs)]

use crate::conf_ext::NgxConfExt;
use crate::config::args_filter::{ArgsFilterDef, InitialPolicy};
use crate::directives::NGX_EMPTY_COMMAND;
use crate::logging::with_config_context;
use crate::nginx_str::NginxStr;
use ngx::core::{NGX_CONF_ERROR, NGX_CONF_OK};
use ngx::ffi::{NGX_CONF_NOARGS, NGX_CONF_TAKE1, NGX_CONF_TAKE2, ngx_command_t, ngx_conf_t};
use tracing::error;

#[unsafe(no_mangle)]
pub static mut ARGS_FILTER_NESTED_COMMANDS: [ngx_command_t; 5] = [
    unsafe { ARGS_FILTER_INITIAL_COMMAND_NESTED },
    unsafe { ARGS_FILTER_EXCLUDE_COMMAND_NESTED },
    unsafe { ARGS_FILTER_INCLUDE_COMMAND_NESTED },
    unsafe { ARGS_FILTER_VOLATILE_COMMAND_NESTED },
    NGX_EMPTY_COMMAND,
];

#[unsafe(no_mangle)]
static mut ARGS_FILTER_INITIAL_COMMAND_NESTED: ngx_command_t = ngx_command_t {
    name: ngx::ngx_string!("initial"),
    type_: NGX_CONF_TAKE1 as _,
    set: Some(args_filter_initial_set),
    conf: 0,
    offset: 0,
    post: core::ptr::null_mut(),
};

#[unsafe(no_mangle)]
static mut ARGS_FILTER_EXCLUDE_COMMAND_NESTED: ngx_command_t = ngx_command_t {
    name: ngx::ngx_string!("exclude"),
    type_: (NGX_CONF_TAKE1 | NGX_CONF_TAKE2) as _,
    set: Some(args_filter_exclude_set),
    conf: 0,
    offset: 0,
    post: core::ptr::null_mut(),
};

#[unsafe(no_mangle)]
static mut ARGS_FILTER_INCLUDE_COMMAND_NESTED: ngx_command_t = ngx_command_t {
    name: ngx::ngx_string!("include"),
    type_: (NGX_CONF_TAKE1 | NGX_CONF_TAKE2) as _,
    set: Some(args_filter_include_set),
    conf: 0,
    offset: 0,
    post: core::ptr::null_mut(),
};

#[unsafe(no_mangle)]
static mut ARGS_FILTER_VOLATILE_COMMAND_NESTED: ngx_command_t = ngx_command_t {
    name: ngx::ngx_string!("volatile"),
    type_: NGX_CONF_NOARGS as _,
    set: Some(args_filter_volatile_set),
    conf: 0,
    offset: 0,
    post: core::ptr::null_mut(),
};

unsafe fn get_current_filter(cf: *mut ngx_conf_t) -> *mut ArgsFilterDef {
    unsafe { (*cf).handler_conf.cast::<ArgsFilterDef>() }
}

#[unsafe(no_mangle)]
extern "C" fn args_filter_initial_set(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    _conf: *mut core::ffi::c_void,
) -> *mut core::ffi::c_char {
    with_config_context(cf, || {
        let cf_ref = unsafe { cf.as_mut().expect("cf") };
        let args = cf_ref.args();
        let filter = unsafe { &mut *get_current_filter(cf) };

        if args.len() != 2 {
            error!(r#"invalid number of arguments in "initial" directive"#);
            return NGX_CONF_ERROR;
        }

        if filter.initial_set {
            error!(r#""initial" directive is duplicate"#);
            return NGX_CONF_ERROR;
        }

        let value = unsafe { std::slice::from_raw_parts(args[1].data, args[1].len) };
        filter.initial = if value == b"all" {
            InitialPolicy::All
        } else if value == b"none" {
            InitialPolicy::None
        } else {
            error!(r#""initial" must be "all" or "none""#);
            return NGX_CONF_ERROR;
        };

        filter.initial_set = true;
        NGX_CONF_OK
    })
}

#[unsafe(no_mangle)]
extern "C" fn args_filter_include_set(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    _conf: *mut core::ffi::c_void,
) -> *mut core::ffi::c_char {
    with_config_context(cf, || {
        let cf_ref = unsafe { cf.as_mut().expect("cf") };
        let args = cf_ref.args();
        let filter = unsafe { &mut *get_current_filter(cf) };

        if args.len() != 2 && args.len() != 3 {
            error!(r#"invalid number of arguments in "include" directive"#);
            return NGX_CONF_ERROR;
        }

        if args.len() == 2 {
            let Ok(key) = NginxStr::from_ngx_str(cf_ref, &args[1]) else {
                error!("failed to allocate include key");
                return NGX_CONF_ERROR;
            };

            filter.add_include_literal(cf_ref.pool(), key);
            return NGX_CONF_OK;
        }

        let mode = unsafe { std::slice::from_raw_parts(args[1].data, args[1].len) };
        let case_insensitive = if mode == b"~" {
            false
        } else if mode == b"~*" {
            true
        } else {
            error!(r#""include" expects literal, "~", or "~*""#);
            return NGX_CONF_ERROR;
        };

        if filter
            .add_include_regex(cf, args[2], case_insensitive)
            .is_err()
        {
            return NGX_CONF_ERROR;
        }

        NGX_CONF_OK
    })
}

#[unsafe(no_mangle)]
extern "C" fn args_filter_exclude_set(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    _conf: *mut core::ffi::c_void,
) -> *mut core::ffi::c_char {
    with_config_context(cf, || {
        let cf_ref = unsafe { cf.as_mut().expect("cf") };
        let args = cf_ref.args();
        let filter = unsafe { &mut *get_current_filter(cf) };

        if args.len() != 2 && args.len() != 3 {
            error!(r#"invalid number of arguments in "exclude" directive"#);
            return NGX_CONF_ERROR;
        }

        if args.len() == 2 {
            let Ok(key) = NginxStr::from_ngx_str(cf_ref, &args[1]) else {
                error!("failed to allocate exclude key");
                return NGX_CONF_ERROR;
            };

            filter.add_exclude_literal(cf_ref.pool(), key);
            return NGX_CONF_OK;
        }

        let mode = unsafe { std::slice::from_raw_parts(args[1].data, args[1].len) };
        let case_insensitive = if mode == b"~" {
            false
        } else if mode == b"~*" {
            true
        } else {
            error!(r#""exclude" expects literal, "~", or "~*""#);
            return NGX_CONF_ERROR;
        };

        if filter
            .add_exclude_regex(cf, args[2], case_insensitive)
            .is_err()
        {
            return NGX_CONF_ERROR;
        }

        NGX_CONF_OK
    })
}

#[unsafe(no_mangle)]
extern "C" fn args_filter_volatile_set(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    _conf: *mut core::ffi::c_void,
) -> *mut core::ffi::c_char {
    with_config_context(cf, || {
        let cf_ref = unsafe { cf.as_mut().expect("cf") };
        let args = cf_ref.args();
        let filter = unsafe { &mut *get_current_filter(cf) };

        if args.len() != 1 {
            error!(r#"invalid number of arguments in "volatile" directive"#);
            return NGX_CONF_ERROR;
        }

        filter.volatile = true;
        NGX_CONF_OK
    })
}
