# Build and Development Guide

This repository builds a Rust-based NGINX dynamic module and validates behavior with workspace tests.

GitHub Actions validates the workspace on `ubuntu-latest` and `macos-latest` using:

- Linux baseline: `cargo test --workspace --exclude integration-tests -- --nocapture`
- Linux integration matrix (all required):  `cargo test -p integration-tests -- --nocapture` with multiple NGINX versions, starting at `NGX_VERSION=1.15.0`
- macOS representative run: `cargo test --workspace -- --nocapture`

## Supported Build Platforms

- Linux
- macOS

## Build Artifacts

- Linux: `target/release/libngx_args_filter_module.so`
- macOS: `target/release/libngx_args_filter_module.dylib`

## Dependencies

### Common

- Rust toolchain (`rustup`, `cargo`)
- `make`
- `pkg-config`

### Linux (Debian/Ubuntu-style package names)

- `clang` or `gcc`
- `libclang-dev`
- `libpcre2-dev`
- `libssl-dev`
- `zlib1g-dev`

### macOS

- Xcode Command Line Tools (`clang`, `make`)
- Accepted Xcode license (`sudo xcodebuild -license`)
- Homebrew packages:
  - `openssl@3`
  - `pcre`

Note: if your environment fails during signature/key operations while preparing vendored sources, install `gnupg`.

## Build Commands

Canonical command:

```bash
bash ./scripts/build.sh
```

Optional convenience command (runs the same build flow through the Cargo alias):

```bash
cargo build-module
```

## Local Validation Commands

```bash
cargo clippy --workspace --all-targets
cargo test --workspace -- --nocapture
```

### Local NGINX Version Matrix Check

Use this to mirror the Linux CI compatibility checks locally:

```bash
for v in 1.15.0 1.18.0 1.22.1 1.28.2 1.29.5; do
  echo "Testing NGX_VERSION=${v}"
  NGX_VERSION="${v}" NGX_CFLAGS="-Wno-deprecated-declarations" \
    cargo test -p integration-tests -- --nocapture
done
```

## Build Environment Variables

- `NGX_VERSION`: target NGINX version. If unset, `scripts/build.sh` auto-detects from local `nginx -v` when available, otherwise uses the script default.
- `NGX_CFLAGS`: extra C flags passed to the NGINX build.
- `NGX_LDFLAGS`: extra linker flags passed to the NGINX build.
- `OPENSSL_PREFIX`: OpenSSL prefix override for macOS (only needed when auto-detection fails).
- `PCRE_PREFIX`: PCRE prefix override for macOS (only needed when auto-detection fails).
- `MAKE`: make command used by `nginx-src`.
- `CARGO_TARGET_DIR`: Cargo output directory.

Examples:

```bash
NGX_VERSION=1.28.1 bash ./scripts/build.sh
OPENSSL_PREFIX=/opt/homebrew/opt/openssl@3 bash ./scripts/build.sh
PCRE_PREFIX=/opt/homebrew/opt/pcre bash ./scripts/build.sh
```

## Troubleshooting

### Signature tooling (`gpg`/`dirmngr`) for vendored source verification

Symptom:

- builds fail while importing verification keys or checking source signatures.

Actions:

- install required tooling (`gnupg` package, including `gpg` and `dirmngr`).
- verify availability:

```bash
command -v gpg
command -v dirmngr
```

For constrained local troubleshooting only (not recommended for CI), you can disable signature checks:

```bash
NGX_NO_SIGNATURE_CHECK=1 NGX_VERSION=1.28.2 bash ./scripts/build.sh
```

### macOS SDK sysroot errors (`sys/types.h` not found)

Symptom:

- build fails in `nginx-sys`/bindgen with `fatal error: 'sys/types.h' file not found`.

Actions:

- Ensure Xcode Command Line Tools are installed and selected.
- Verify SDK path: `xcrun --sdk macosx --show-sdk-path`
- Confirm header exists: `${SDKROOT}/usr/include/sys/types.h`
- If needed, export bindgen sysroot args before building:

```bash
export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$(xcrun --sdk macosx --show-sdk-path)"
bash ./scripts/build.sh
```

### macOS socket path too long during key/signature steps

Symptom:

- failures including `S.dirmngr ... File name too long`.

Action:

- keep target/output paths short (for example `export CARGO_TARGET_DIR=/tmp/ngxaf-target`).

### macOS Xcode license not accepted

Symptom:

- linker/build failure mentioning Xcode license.

Action:

```bash
sudo xcodebuild -license
```

### macOS OpenSSL/PCRE not found

Symptom:

- configure errors for OpenSSL or rewrite/PCRE.

Actions:

```bash
brew install openssl@3 pcre
bash ./scripts/build.sh
```

If installed in non-default paths, set `OPENSSL_PREFIX` and `PCRE_PREFIX`.

### NGINX/module version mismatch at runtime

Symptom:

- `dlopen ... symbol not found ...` when loading the module.

Action:

- build against the same NGINX version as runtime:

```bash
NGX_VERSION="$(nginx -v 2>&1 | sed -n 's#.*nginx/\([0-9][0-9.]*\).*#\1#p')" bash ./scripts/build.sh
```
