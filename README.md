# ngx-args-filter-module

[![Mac Tests (main)](https://img.shields.io/github/actions/workflow/status/ghostnumber7/ngx-args-filter-module/mac-tests.yml?branch=main&label=Mac%20Tests&cacheSeconds=60)](https://github.com/ghostnumber7/ngx-args-filter-module/actions/workflows/mac-tests.yml)
[![Linux Tests (main)](https://img.shields.io/github/actions/workflow/status/ghostnumber7/ngx-args-filter-module/linux-tests.yml?branch=main&label=Linux%20Tests&cacheSeconds=60)](https://github.com/ghostnumber7/ngx-args-filter-module/actions/workflows/linux-tests.yml)

`ngx-args-filter-module` is an HTTP dynamic module for [NGINX](https://nginx.org/) written in Rust with the [ngx](https://crates.io/crates/ngx) framework.

It builds filtered query-string variables so you can forward only the parameters you intend to keep.

NGINX exposes `$args` (full query string) and `$arg_<name>` (single known keys), but it does not provide a native way to compose a filtered query string when allowed keys are dynamic or partially unknown. This module closes that gap with declarative include/exclude rules.

## When to use it

Use this module when you need to:

- Keep only a controlled subset of query parameters.
- Remove sensitive tokens before proxying.
- Apply include/exclude rules with literal or regex matching.
- Preserve original ordering and raw bytes for kept segments.

## Quick Start

1. Build the module:
   - `bash ./scripts/build.sh`
2. Load it in `nginx.conf`:
   - Linux: `load_module /etc/nginx/modules/libngx_args_filter_module.so;`
   - macOS: `load_module /usr/local/etc/nginx/modules/libngx_args_filter_module.dylib;`
3. Define and use a filtered variable:

```nginx
args_filter $filtered_args {
    initial all;
    exclude ~ "^ads\.";
    include ads.test;
}

server {
    location / {
        return 200 "$filtered_args";
    }
}
```

## Example Configurations

### 1. Reverse proxy to AWS MediaTailor with controlled passthrough

```nginx
args_filter $mediatailor_args {
    initial none;
    include ~ "^aws\.";
    include ~ "^playerParams\.";
}

location /v1/master/ {
    proxy_pass https://origin.mediatailor.region.amazonaws.com/v1/master/$mediatailor_args;
}
```

### 2. Strip auth-related query params before upstream

```nginx
args_filter $safe_upstream_args {
    initial all;
    exclude secure_link;
    exclude token;
    exclude auth;
    exclude signature;
}

location /api/ {
    proxy_pass https://api.example.com$request_uri?$safe_upstream_args;
}
```

## Supported Platforms

- Linux
- macOS

## Build Dependencies

For complete setup and troubleshooting, see [docs/build.md](docs/build.md).

High-level requirements:

- Rust toolchain (`rustup`, `cargo`)
- Build tools (`make`, `pkg-config`)
- Linux: `clang` or `gcc`, plus development headers for `libclang`, `pcre2`, OpenSSL, and zlib
- macOS: Xcode Command Line Tools plus Homebrew packages `openssl@3` and `pcre`

## Documentation

- [Build and development guide](docs/build.md)
- [Module directive reference](docs/module.md)
- [Performance methodology](docs/performance.md)

## Known Limitations

- `args_filter` currently registers in the NGINX `http` main context.
- Matching uses raw key bytes; query keys are not percent-decoded before matching.

## Contributing

Contribution and review workflow is documented in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under [Apache License 2.0](LICENSE).

Copyright 2026 Marco Godoy.
