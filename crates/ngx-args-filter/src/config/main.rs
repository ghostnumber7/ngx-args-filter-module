//! Main configuration structure.

use crate::config::args_filter::ArgsFilterDef;
use crate::nginx_str::NginxStr;
use ngx::collections::rbtree::RbTreeMap;
use ngx::core::Pool;
use std::fmt;

/// Module main configuration.
#[derive(Default)]
pub struct MainConf {
    /// Map of `args_filter` variable names to compiled definitions.
    pub args_filters: Option<RbTreeMap<NginxStr<Pool>, ArgsFilterDef, Pool>>,
}

impl fmt::Debug for MainConf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MainConf")
            .field(
                "args_filters_count",
                &self.args_filters.as_ref().map(|m| m.iter().count()),
            )
            .finish()
    }
}
