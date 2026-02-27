//! Compile-time nginx version helpers.

/// Parse an ASCII decimal version number produced by `build.rs`.
pub const fn parse_version_number(value: &str) -> usize {
    let bytes = value.as_bytes();
    let mut out = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        assert!(
            b >= b'0' && b <= b'9',
            "NGX_VERSION_NUMBER must contain only digits"
        );
        out = out * 10 + (b - b'0') as usize;
        i += 1;
    }

    out
}

pub const NGX_VERSION_NUMBER: usize = parse_version_number(env!("NGX_VERSION_NUMBER"));

#[cfg(test)]
mod tests {
    use super::parse_version_number;

    #[test]
    fn parses_expected_number() {
        assert_eq!(parse_version_number("1028001"), 1_028_001);
    }
}
