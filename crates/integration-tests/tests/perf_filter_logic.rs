use std::time::{Duration, Instant};

fn filter_args_by<F>(args: &[u8], mut keep_key: F) -> Vec<u8>
where
    F: FnMut(&[u8]) -> bool,
{
    let mut output = Vec::with_capacity(args.len());

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

fn p50_p95(durations: &mut [Duration]) -> (Duration, Duration) {
    durations.sort_unstable();
    let len = durations.len();
    let p50 = durations[len / 2];
    let p95 = durations[(len * 95) / 100];
    (p50, p95)
}

#[test]
fn perf_filter_args_harness() {
    // Representative mixed payload: literals, repeated keys, and regex-like prefixes.
    let args = b"x=1&ads.foo=2&ads.test=3&y=4&utm_source=a&utm_medium=b&test[]=1&test[]=2&a=9&b=10";

    let iterations = 30_000usize;
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let _out = filter_args_by(args, |k| {
            k == b"x" || k == b"y" || k == b"ads.test" || k == b"test[]" || k.starts_with(b"utm_")
        });
        samples.push(start.elapsed());
    }

    let (p50, p95) = p50_p95(&mut samples);
    println!("perf_filter_args_harness: iterations={iterations} p50={p50:?} p95={p95:?}");
}
