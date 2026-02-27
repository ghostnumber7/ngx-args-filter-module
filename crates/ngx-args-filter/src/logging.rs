//! Tracing integration for nginx logging.
//!
//! Events are forwarded to the nginx logger tied to the current config/request context.

use std::cell::Cell;
use tracing::{Event, Level, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context};

/// Nginx context type - either config-time or request-time
#[derive(Clone, Copy)]
pub enum NginxContext {
    /// Configuration context (during config parsing)
    Config(*mut ngx::ffi::ngx_conf_t),
    /// Request context (during request handling)
    Request(*mut ngx::ffi::ngx_log_t),
}

thread_local! {
    static NGX_CONTEXT: Cell<Option<NginxContext>> = const { Cell::new(None) };
}

const fn level_to_ngx(level: Level) -> ngx::ffi::ngx_uint_t {
    match level {
        Level::ERROR => ngx::ffi::NGX_LOG_ERR as _,
        Level::WARN => ngx::ffi::NGX_LOG_WARN as _,
        Level::INFO => ngx::ffi::NGX_LOG_INFO as _,
        Level::DEBUG | Level::TRACE => ngx::ffi::NGX_LOG_DEBUG as _,
    }
}

pub struct NginxTracingLayer;

impl<S> Layer<S> for NginxTracingLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        struct MessageVisitor(Option<String>);

        impl Visit for MessageVisitor {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if field.name() == "message" {
                    self.0 = Some(value.to_owned());
                }
            }

            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = Some(format!("{value:?}"));
                }
            }
        }

        let mut visitor = MessageVisitor(None);
        event.record(&mut visitor);

        let final_message = visitor
            .0
            .unwrap_or_else(|| event.metadata().name().to_string());
        let level = level_to_ngx(*event.metadata().level());
        let context = NGX_CONTEXT.with(std::cell::Cell::get);

        match context {
            Some(NginxContext::Request(log)) if !log.is_null() => {
                ngx::ngx_log_error!(level, log, "{}", final_message);
            }
            Some(NginxContext::Config(cf)) if !cf.is_null() => {
                let Ok(c_msg) = std::ffi::CString::new(final_message.clone()) else {
                    eprintln!("{final_message}");
                    return;
                };
                unsafe {
                    ngx::ffi::ngx_conf_log_error(
                        level,
                        cf,
                        0,
                        c_msg.as_ptr().cast::<core::ffi::c_char>(),
                    );
                }
            }
            _ => eprintln!("{final_message}"),
        }
    }
}

static TRACING_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize tracing subscriber that forwards to nginx logs.
pub fn init_tracing_subscriber() {
    TRACING_INIT.call_once(|| {
        use tracing_subscriber::prelude::*;

        let nginx_layer = NginxTracingLayer;
        let subscriber = tracing_subscriber::registry().with(nginx_layer);
        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}

struct ContextGuard {
    previous: Option<NginxContext>,
}

impl ContextGuard {
    fn set(context: NginxContext) -> Self {
        NGX_CONTEXT.with(|ctx| {
            let previous = ctx.get();
            ctx.set(Some(context));
            Self { previous }
        })
    }
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        NGX_CONTEXT.with(|ctx| {
            ctx.set(self.previous);
        });
    }
}

/// Execute a closure with config context.
pub fn with_config_context<F, R>(cf: *mut ngx::ffi::ngx_conf_t, f: F) -> R
where
    F: FnOnce() -> R,
{
    init_tracing_subscriber();
    let _guard = ContextGuard::set(NginxContext::Config(cf));
    f()
}

/// Execute a closure with request context.
pub fn with_request_context<F, R>(log: *mut ngx::ffi::ngx_log_t, f: F) -> R
where
    F: FnOnce() -> R,
{
    init_tracing_subscriber();
    let _guard = ContextGuard::set(NginxContext::Request(log));
    f()
}
