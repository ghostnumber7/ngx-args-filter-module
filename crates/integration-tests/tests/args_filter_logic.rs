fn apply_filter<F>(args: &str, mut keep_key: F) -> String
where
    F: FnMut(&[u8]) -> bool,
{
    let mut out = Vec::with_capacity(args.len());

    for segment in args.as_bytes().split(|b| *b == b'&') {
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

        if !out.is_empty() {
            out.push(b'&');
        }
        out.extend_from_slice(segment);
    }

    String::from_utf8(out).unwrap()
}

#[test]
fn keeps_only_selected_keys_and_preserves_segment_bytes() {
    let out = apply_filter("x=1&a=2&b=%2B%20&y=4", |k| k == b"a" || k == b"b");
    assert_eq!(out, "a=2&b=%2B%20");
}

#[test]
fn preserves_input_order_of_kept_segments() {
    let out = apply_filter("b=2&a=1&c=3", |k| k == b"c" || k == b"b");
    assert_eq!(out, "b=2&c=3");
}

#[test]
fn handles_missing_values_and_empty_segments() {
    let out = apply_filter("&&a&&b=2&c&&", |k| k == b"a" || k == b"c");
    assert_eq!(out, "a&c");
}

#[test]
fn preserves_repeated_array_like_keys_in_original_order() {
    let out = apply_filter("a=0&test[]=1&b=2&test[]=3&test[]=4", |k| k == b"test[]");
    assert_eq!(out, "test[]=1&test[]=3&test[]=4");
}

#[test]
fn treats_percent_encoded_keys_as_raw_bytes_without_decoding() {
    let out = apply_filter("test%5B%5D=1&test[]=2", |k| k == b"test[]");
    assert_eq!(out, "test[]=2");
}

#[test]
fn preserves_percent_encoded_plus_in_values() {
    let out = apply_filter("keep=%2B&drop=x+y&keep2=a%2Bb", |k| {
        k == b"keep" || k == b"keep2"
    });
    assert_eq!(out, "keep=%2B&keep2=a%2Bb");
}
