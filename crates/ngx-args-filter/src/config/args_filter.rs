//! `args_filter` configuration structures and evaluation logic

use crate::nginx_str::NginxStr;
use crate::status::NgxStatus;
use ngx::collections::Vec;
use ngx::core::Pool;
use ngx::ffi::{NGX_PCRE, NGX_REGEX_CASELESS, ngx_regex_compile_t, ngx_str_t};
use tracing::{debug, error};

#[cfg(ngx_feature = "pcre2")]
use ngx::ffi::ngx_regex_exec;
#[cfg(not(ngx_feature = "pcre2"))]
use ngx::ffi::pcre_exec;

/// Default behavior when a key does not match any include/exclude rule.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InitialPolicy {
    /// Keep no keys unless rules include them.
    #[default]
    None,
    /// Keep all keys unless rules exclude them.
    All,
}

/// Runtime data attached to NGINX variable registration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArgsFilterVarData {
    pub name: ngx_str_t,
}

#[derive(Clone, Copy, Debug)]
pub enum RuleAction {
    Include,
    Exclude,
}

#[derive(Clone, Copy, Debug)]
pub struct CompiledRegex {
    pub regex: *mut ngx::ffi::ngx_regex_t,
}

#[derive(Debug)]
pub enum RuleMatcher {
    Literal(NginxStr<Pool>),
    Regex(CompiledRegex),
}

#[derive(Debug)]
pub struct Rule {
    pub action: RuleAction,
    pub matcher: RuleMatcher,
}

/// Full configuration for one `args_filter` variable.
#[derive(Debug, Default)]
pub struct ArgsFilterDef {
    pub initial: InitialPolicy,
    pub initial_set: bool,
    /// If true, mark the exposed nginx variable as non-cacheable.
    pub volatile: bool,
    pub rules: Option<Vec<Rule, Pool>>,
}

impl ArgsFilterDef {
    /// Create a new filter definition with default semantics.
    pub const fn new() -> Self {
        Self {
            initial: InitialPolicy::None,
            initial_set: false,
            volatile: false,
            rules: None,
        }
    }

    /// Return true when `key` should be kept.
    /// Rules are evaluated in declaration order.
    pub fn should_keep_key(&self, key: &[u8]) -> bool {
        let mut keep = self.initial == InitialPolicy::All;
        let key_text = String::from_utf8_lossy(key);

        let Some(rules) = self.rules.as_ref() else {
            debug!(
                "args_filter: key='{}' no rules configured; keep={}",
                key_text, keep
            );
            return keep;
        };

        debug!(
            "args_filter: key='{}' evaluating {} rules; initial_keep={}",
            key_text,
            rules.len(),
            keep
        );

        for (idx, rule) in rules.iter().enumerate() {
            if !rule.matches(key) {
                debug!(
                    "args_filter: key='{}' rule[{}] {} did not match",
                    key_text,
                    idx,
                    rule.debug_label()
                );
                continue;
            }

            keep = match rule.action {
                RuleAction::Include => true,
                RuleAction::Exclude => false,
            };
            debug!(
                "args_filter: key='{}' rule[{}] {} matched; keep={}",
                key_text,
                idx,
                rule.debug_label(),
                keep
            );
        }

        debug!("args_filter: key='{}' final keep={}", key_text, keep);
        keep
    }

    /// Returns true when output is always identical to input query args.
    pub fn is_identity_filter(&self) -> bool {
        self.initial == InitialPolicy::All
            && self
                .rules
                .as_ref()
                .is_none_or(ngx::collections::Vec::is_empty)
    }

    pub fn add_include_literal(&mut self, pool: Pool, key: NginxStr<Pool>) {
        self.push_rule(pool, RuleAction::Include, RuleMatcher::Literal(key));
    }

    pub fn add_exclude_literal(&mut self, pool: Pool, key: NginxStr<Pool>) {
        self.push_rule(pool, RuleAction::Exclude, RuleMatcher::Literal(key));
    }

    pub fn add_include_regex(
        &mut self,
        cf: *mut ngx::ffi::ngx_conf_t,
        pattern: ngx_str_t,
        case_insensitive: bool,
    ) -> Result<(), ()> {
        let regex = compile_regex(cf, pattern, case_insensitive)?;
        let pool = unsafe { Pool::from_ngx_pool((*cf).pool) };
        self.push_rule(pool, RuleAction::Include, RuleMatcher::Regex(regex));
        Ok(())
    }

    pub fn add_exclude_regex(
        &mut self,
        cf: *mut ngx::ffi::ngx_conf_t,
        pattern: ngx_str_t,
        case_insensitive: bool,
    ) -> Result<(), ()> {
        let regex = compile_regex(cf, pattern, case_insensitive)?;
        let pool = unsafe { Pool::from_ngx_pool((*cf).pool) };
        self.push_rule(pool, RuleAction::Exclude, RuleMatcher::Regex(regex));
        Ok(())
    }

    fn push_rule(&mut self, pool: Pool, action: RuleAction, matcher: RuleMatcher) {
        if self.rules.is_none() {
            self.rules = Some(Vec::new_in(pool));
        }

        if let Some(rules) = self.rules.as_mut() {
            rules.push(Rule { action, matcher });
        }
    }
}

impl Rule {
    fn matches(&self, key: &[u8]) -> bool {
        match &self.matcher {
            RuleMatcher::Literal(expected) => expected.as_bytes() == key,
            RuleMatcher::Regex(regex) => regex_matches(regex, key),
        }
    }

    const fn debug_label(&self) -> &'static str {
        match (self.action, &self.matcher) {
            (RuleAction::Include, RuleMatcher::Literal(_)) => "include literal",
            (RuleAction::Exclude, RuleMatcher::Literal(_)) => "exclude literal",
            (RuleAction::Include, RuleMatcher::Regex(_)) => "include regex",
            (RuleAction::Exclude, RuleMatcher::Regex(_)) => "exclude regex",
        }
    }
}

fn compile_regex(
    cf: *mut ngx::ffi::ngx_conf_t,
    pattern: ngx_str_t,
    case_insensitive: bool,
) -> Result<CompiledRegex, ()> {
    if NGX_PCRE == 0 {
        error!("regex rules require NGINX with PCRE/PCRE2 support");
        return Err(());
    }

    let mut err_buf = [0u8; 256];
    let mut rc = unsafe { core::mem::zeroed::<ngx_regex_compile_t>() };
    rc.pattern = pattern;
    rc.pool = unsafe { (*cf).pool };
    rc.options = if case_insensitive {
        NGX_REGEX_CASELESS as _
    } else {
        0
    };
    rc.err = ngx_str_t {
        len: err_buf.len(),
        data: err_buf.as_mut_ptr(),
    };

    if unsafe { ngx::ffi::ngx_regex_compile(&raw mut rc) } != NgxStatus::OK {
        let err_len = err_buf
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(err_buf.len());
        let err_text = String::from_utf8_lossy(&err_buf[..err_len]);
        error!("failed to compile regex: {}", err_text);
        return Err(());
    }

    if rc.regex.is_null() {
        error!("failed to compile regex: empty compiled regex pointer");
        return Err(());
    }

    Ok(CompiledRegex { regex: rc.regex })
}

fn regex_matches(regex: &CompiledRegex, key: &[u8]) -> bool {
    let key_ngx = ngx_str_t {
        len: key.len(),
        data: key.as_ptr().cast_mut(),
    };

    #[cfg(ngx_feature = "pcre2")]
    let rc: ngx::ffi::ngx_int_t = unsafe {
        let mut key_ngx = key_ngx;
        ngx_regex_exec(regex.regex, &raw mut key_ngx, core::ptr::null_mut(), 0)
    };

    #[cfg(not(ngx_feature = "pcre2"))]
    let rc: ngx::ffi::ngx_int_t = unsafe {
        pcre_exec(
            (*regex.regex).code,
            (*regex.regex).extra,
            key_ngx.data.cast(),
            key_ngx.len as core::ffi::c_int,
            0,
            0,
            core::ptr::null_mut(),
            0,
        ) as _
    };

    if rc >= 0 {
        return true;
    }

    if rc != NgxStatus::DECLINED {
        error!("regex execution failed with rc={}", rc);
    }

    false
}
