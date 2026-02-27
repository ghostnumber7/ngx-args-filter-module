//! `args_filter` block directive implementation.

#![allow(static_mut_refs)]

use crate::NgxArgsFilterModule;
use crate::conf_ext::NgxConfExt;
use crate::config::args_filter::{ArgsFilterDef, ArgsFilterVarData};
use crate::logging::{with_config_context, with_request_context};
use crate::nginx_str::NginxStr;
use crate::status::NgxStatus;
use ngx::core::{NGX_CONF_ERROR, NGX_CONF_OK};
use ngx::ffi::{
    NGX_CONF_BLOCK, NGX_CONF_TAKE1, NGX_HTTP_MAIN_CONF, ngx_command_t, ngx_conf_t,
    ngx_http_add_variable, ngx_http_variable_value_t, ngx_int_t, ngx_pcalloc, ngx_pnalloc,
};
use ngx::http::HttpModuleMainConf;
use tracing::{debug, error};

#[unsafe(no_mangle)]
pub static mut ARGS_FILTER_COMMAND: ngx_command_t = ngx_command_t {
    name: ngx::ngx_string!("args_filter"),
    type_: (NGX_HTTP_MAIN_CONF | NGX_CONF_BLOCK | NGX_CONF_TAKE1) as _,
    set: Some(args_filter_set),
    conf: 0,
    offset: 0,
    post: core::ptr::null_mut(),
};

#[unsafe(no_mangle)]
extern "C" fn args_filter_set(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut core::ffi::c_void,
) -> *mut core::ffi::c_char {
    with_config_context(cf, || {
        let cf_ref = unsafe { cf.as_mut().expect("cf") };
        let main_conf = unsafe {
            conf.cast::<crate::config::MainConf>()
                .as_mut()
                .expect("main_conf")
        };
        let args = cf_ref.args();

        let Ok(var_name) = parse_variable_name(cf_ref, &args[1]) else {
            return NGX_CONF_ERROR;
        };

        if main_conf.args_filters.is_none() {
            let Ok(map) = ngx::collections::RbTreeMap::try_new_in(cf_ref.pool()) else {
                error!("failed to initialize args_filter map");
                return NGX_CONF_ERROR;
            };
            main_conf.args_filters = Some(map);
        }

        let Some(filters_map) = main_conf.args_filters.as_ref() else {
            error!("args_filter map unavailable");
            return NGX_CONF_ERROR;
        };

        if filters_map.get(var_name.as_bytes()).is_some() {
            error!("duplicate args_filter declaration for ${}", var_name);
            return NGX_CONF_ERROR;
        }

        let mut var_name_ngx = var_name.as_ngx_str();
        let var = unsafe { ngx_http_add_variable(cf, &raw mut var_name_ngx, 0) };
        if var.is_null() {
            error!("failed to register variable ${}", var_name);
            return NGX_CONF_ERROR;
        }

        let var_data = unsafe { allocate_var_data(cf, &var_name) };
        if var_data.is_null() {
            error!("failed to allocate args_filter variable metadata");
            return NGX_CONF_ERROR;
        }

        unsafe {
            (*var).get_handler = Some(args_filter_variable_get_handler);
            (*var).data = var_data.cast::<ArgsFilterVarData>() as _;
        }

        let mut filter = ArgsFilterDef::new();

        let mut block_cf = *cf_ref;
        block_cf.handler = Some(args_filter_block_handler);
        block_cf.handler_conf = core::ptr::addr_of_mut!(filter).cast();

        let rv = unsafe { ngx::ffi::ngx_conf_parse(&raw mut block_cf, core::ptr::null_mut()) };
        if rv != NGX_CONF_OK {
            return rv;
        }

        let Some(filters_map_mut) = main_conf.args_filters.as_mut() else {
            error!("args_filter map unavailable after parse");
            return NGX_CONF_ERROR;
        };

        if filters_map_mut.try_insert(var_name, filter).is_err() {
            error!("failed to store args_filter definition");
            return NGX_CONF_ERROR;
        }

        NGX_CONF_OK
    })
}

#[unsafe(no_mangle)]
extern "C" fn args_filter_block_handler(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    _conf: *mut core::ffi::c_void,
) -> *mut core::ffi::c_char {
    use crate::directives::args_filter_nested::ARGS_FILTER_NESTED_COMMANDS;

    with_config_context(cf, || {
        let cf_ref = unsafe { cf.as_mut().expect("cf") };
        let args = cf_ref.args();

        if args.is_empty() {
            error!("args_filter block handler received empty directive");
            return NGX_CONF_ERROR;
        }

        for cmd in unsafe { &ARGS_FILTER_NESTED_COMMANDS[..] } {
            if cmd.name.len == 0 || cmd.set.is_none() {
                continue;
            }

            if cmd.name.len != args[0].len {
                continue;
            }

            let lhs = unsafe { std::slice::from_raw_parts(cmd.name.data, cmd.name.len) };
            let rhs = unsafe { std::slice::from_raw_parts(args[0].data, args[0].len) };
            if lhs != rhs {
                continue;
            }

            let Some(handler) = cmd.set else {
                return NGX_CONF_ERROR;
            };

            let cmd_ptr = std::ptr::from_ref::<ngx_command_t>(cmd).cast_mut();
            return unsafe { handler(cf, cmd_ptr, (*cf).handler_conf) };
        }

        error!("unknown directive inside args_filter block");
        NGX_CONF_ERROR
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn args_filter_variable_get_handler(
    r: *mut ngx::ffi::ngx_http_request_t,
    v: *mut ngx_http_variable_value_t,
    data: usize,
) -> ngx_int_t {
    if r.is_null() || v.is_null() {
        return NgxStatus::ERROR;
    }

    let log = unsafe {
        let conn = (*r).connection;
        if conn.is_null() {
            core::ptr::null_mut()
        } else {
            (*conn).log
        }
    };

    with_request_context(log, || {
        let req = unsafe { ngx::http::Request::from_ngx_http_request(r) };

        let var_data = data as *const ArgsFilterVarData;
        if var_data.is_null() {
            error!("args_filter variable metadata is null");
            return mark_not_found(v);
        }

        let name_slice =
            unsafe { std::slice::from_raw_parts((*var_data).name.data, (*var_data).name.len) };
        let var_name = String::from_utf8_lossy(name_slice);

        let Some(main_conf) = NgxArgsFilterModule::main_conf(req) else {
            error!("failed to fetch module main conf in variable handler");
            return mark_not_found(v);
        };

        let Some(filters) = main_conf.args_filters.as_ref() else {
            return mark_not_found(v);
        };

        let Some(filter) = filters.get(name_slice) else {
            return mark_not_found(v);
        };

        let args = unsafe {
            if (*r).args.len == 0 {
                &[]
            } else {
                std::slice::from_raw_parts((*r).args.data, (*r).args.len)
            }
        };
        let args_text = String::from_utf8_lossy(args);

        debug!(
            "args_filter: evaluating variable='${}' volatile={} args='{}'",
            var_name, filter.volatile, args_text
        );
        if filter.is_identity_filter() {
            debug!(
                "args_filter: variable='${}' using identity fast-path; output unchanged",
                var_name
            );
            return unsafe { set_variable_value(r, v, args, filter.volatile) };
        }

        let filtered = filter_args_by(args, |key| filter.should_keep_key(key));
        debug!(
            "args_filter: variable='${}' filtered result='{}'",
            var_name,
            String::from_utf8_lossy(&filtered)
        );
        unsafe { set_variable_value(r, v, &filtered, filter.volatile) }
    })
}

fn parse_variable_name(
    cf: &ngx_conf_t,
    raw: &ngx::ffi::ngx_str_t,
) -> Result<NginxStr<ngx::core::Pool>, ()> {
    let bytes = unsafe { std::slice::from_raw_parts(raw.data, raw.len) };

    if bytes.is_empty() || bytes[0] != b'$' {
        error!("args_filter variable must start with '$'");
        return Err(());
    }

    let name = &bytes[1..];
    if name.is_empty() {
        error!("args_filter variable name cannot be empty");
        return Err(());
    }

    if !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_') {
        error!("args_filter variable name contains invalid characters");
        return Err(());
    }

    let normalized_name: std::vec::Vec<u8> = name.iter().map(u8::to_ascii_lowercase).collect();

    let pool = unsafe { ngx::core::Pool::from_ngx_pool(cf.pool) };
    NginxStr::from_bytes(pool, &normalized_name).map_err(|_| {
        error!("failed to allocate args_filter variable name");
    })
}

unsafe fn allocate_var_data(
    cf: *mut ngx_conf_t,
    var_name: &NginxStr<ngx::core::Pool>,
) -> *mut core::ffi::c_void {
    let size = core::mem::size_of::<ArgsFilterVarData>();
    let data = unsafe { ngx_pcalloc((*cf).pool, size) };
    if data.is_null() {
        return core::ptr::null_mut();
    }

    let var_data = data.cast::<ArgsFilterVarData>();
    unsafe {
        (*var_data).name = var_name.as_ngx_str();
    }

    data
}

fn filter_args_by<F>(args: &[u8], mut keep_key: F) -> std::vec::Vec<u8>
where
    F: FnMut(&[u8]) -> bool,
{
    let mut output = std::vec::Vec::with_capacity(args.len());

    for segment in args.split(|b| *b == b'&') {
        if segment.is_empty() {
            continue;
        }

        let key_len = segment
            .iter()
            .position(|b| *b == b'=')
            .unwrap_or(segment.len());
        let key = &segment[..key_len];

        if !keep_key(key) {
            continue;
        }

        if !output.is_empty() {
            output.push(b'&');
        }
        output.extend_from_slice(segment);
    }

    output
}

unsafe fn set_variable_value(
    r: *mut ngx::ffi::ngx_http_request_t,
    v: *mut ngx_http_variable_value_t,
    value: &[u8],
    no_cacheable: bool,
) -> ngx_int_t {
    unsafe {
        (*v).set_valid(1);
        (*v).set_not_found(0);
        (*v).set_no_cacheable(u32::from(no_cacheable));
    }

    if value.is_empty() {
        unsafe {
            (*v).set_len(0);
            (*v).data = core::ptr::null_mut();
        }
        return NgxStatus::OK;
    }

    let dst = unsafe { ngx_pnalloc((*r).pool, value.len()) }.cast::<u8>();
    if dst.is_null() {
        return NgxStatus::ERROR;
    }

    unsafe {
        core::ptr::copy_nonoverlapping(value.as_ptr(), dst, value.len());
        let Ok(len) = u32::try_from(value.len()) else {
            return NgxStatus::ERROR;
        };
        (*v).set_len(len);
        (*v).data = dst;
    }

    NgxStatus::OK
}

fn mark_not_found(v: *mut ngx_http_variable_value_t) -> ngx_int_t {
    unsafe {
        (*v).set_valid(0);
        (*v).set_not_found(1);
        (*v).set_no_cacheable(0);
        (*v).set_len(0);
        (*v).data = core::ptr::null_mut();
    }

    NgxStatus::OK
}

#[cfg(test)]
mod tests {
    use super::filter_args_by;

    #[test]
    fn filter_args_keeps_expected_keys() {
        let out = filter_args_by(b"x=1&ads.foo=2&ads.test=3&y=4", |k| {
            k == b"x" || k == b"ads.test" || k == b"y"
        });
        assert_eq!(out, b"x=1&ads.test=3&y=4");
    }

    #[test]
    fn filter_args_handles_missing_values_and_separators() {
        let out = filter_args_by(b"&&a&b=2&&c", |k| k == b"a" || k == b"c");
        assert_eq!(out, b"a&c");
    }

    #[test]
    fn filter_args_preserves_percent_encoded_plus_bytes() {
        let out = filter_args_by(b"keep=%2B&drop=x+y&keep2=a%2Bb", |k| {
            k == b"keep" || k == b"keep2"
        });
        assert_eq!(out, b"keep=%2B&keep2=a%2Bb");
    }
}
