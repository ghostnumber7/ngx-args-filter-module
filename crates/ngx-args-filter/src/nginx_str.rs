//! Safe wrapper for nginx string types.
//!
//! `NginxStr` owns pool-allocated bytes and can be used safely as an `RbTreeMap` key.

use ngx::collections::{TryReserveError, Vec};
use ngx::core::Pool;
use ngx::ffi::{ngx_conf_t, ngx_str_t};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Pool-allocated nginx string wrapper.
#[derive(Clone)]
pub struct NginxStr<A: ngx::allocator::Allocator> {
    bytes: Vec<u8, A>,
}

impl<A: ngx::allocator::Allocator> NginxStr<A> {
    /// Create a new `NginxStr` from bytes using the given allocator.
    pub fn from_bytes(alloc: A, bytes: &[u8]) -> Result<Self, TryReserveError> {
        let mut vec = Vec::new_in(alloc);
        vec.try_reserve_exact(bytes.len())?;
        vec.extend_from_slice(bytes);
        Ok(Self { bytes: vec })
    }

    /// Get the underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    /// Convert to `ngx_str_t` for FFI calls.
    ///
    /// The returned pointer is borrowed from `self`, so the resulting `ngx_str_t`
    /// is only valid while `self` is alive and unmoved.
    pub fn as_ngx_str(&self) -> ngx_str_t {
        ngx_str_t {
            data: self.bytes.as_ptr().cast_mut(),
            len: self.bytes.len(),
        }
    }
}

impl NginxStr<Pool> {
    /// Create `NginxStr` by copying bytes from nginx config args into config pool memory.
    pub fn from_ngx_str(cf: &ngx_conf_t, src: &ngx_str_t) -> Result<Self, TryReserveError> {
        let pool = unsafe { Pool::from_ngx_pool(cf.pool) };
        let bytes = unsafe { std::slice::from_raw_parts(src.data, src.len) };
        Self::from_bytes(pool, bytes)
    }
}

impl<A: ngx::allocator::Allocator> Borrow<[u8]> for NginxStr<A> {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<A: ngx::allocator::Allocator> PartialEq for NginxStr<A> {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<A: ngx::allocator::Allocator> Eq for NginxStr<A> {}

impl<A: ngx::allocator::Allocator> PartialOrd for NginxStr<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: ngx::allocator::Allocator> Ord for NginxStr<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl<A: ngx::allocator::Allocator> Hash for NginxStr<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl<A: ngx::allocator::Allocator> fmt::Debug for NginxStr<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bytes.is_empty() {
            return write!(f, "<empty>");
        }

        write!(f, "\"{}\"", String::from_utf8_lossy(self.as_bytes()))
    }
}

impl<A: ngx::allocator::Allocator> fmt::Display for NginxStr<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.as_bytes()))
    }
}
