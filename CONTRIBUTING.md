# Contributing

Thanks for contributing to `ngx-args-filter-module`.

## Before opening an issue

Please include:

- What you expected to happen.
- What happened instead.
- NGINX version, OS, and exact configuration snippet.
- Reproduction steps and relevant logs.

## Pull request checklist

1. Keep changes focused and scoped.
2. Prefer self-explanatory code; remove comments that only restate obvious behavior.
3. Add or update tests when behavior changes.
4. Run local checks before opening the PR.
5. Keep docs and examples aligned with the code changes.

## Required local checks

```bash
cargo clippy --workspace --all-targets
cargo test --workspace -- --nocapture
```

## Style and review expectations

- Follow existing Rust style and lint expectations.
- Avoid unnecessary abstractions and non-idiomatic workarounds.
- Prefer standard library or crate-supported patterns over custom implementations.
- Keep commit history and PR description clear enough for external reviewers.
