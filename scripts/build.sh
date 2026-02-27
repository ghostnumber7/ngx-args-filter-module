#!/usr/bin/env bash

set -euo pipefail

DEFAULT_NGX_VERSION="1.28.1"

append_flag_once() {
    local var_name="$1"
    local flag="$2"
    local current="${!var_name:-}"

    case " ${current} " in
        *" ${flag} "*) ;;
        *)
            if [ -z "$current" ]; then
                export "${var_name}=${flag}"
            else
                export "${var_name}=${current} ${flag}"
            fi
            ;;
    esac
}

if [ "$(uname -s)" = "Darwin" ]; then
    if [ -z "${MAKE:-}" ] && command -v gmake >/dev/null 2>&1; then
        export MAKE="gmake"
    fi

    SDK_PATH="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || true)"
    if [ -n "$SDK_PATH" ]; then
        export SDKROOT="$SDK_PATH"
        append_flag_once NGX_CFLAGS "-isysroot ${SDKROOT}"
    fi

    OPENSSL_PREFIX="${OPENSSL_PREFIX:-}"
    if [ -z "$OPENSSL_PREFIX" ] && command -v brew >/dev/null 2>&1; then
        OPENSSL_PREFIX="$(brew --prefix openssl@3 2>/dev/null || true)"
    fi
    if [ -z "$OPENSSL_PREFIX" ] && [ -d "/opt/homebrew/opt/openssl@3" ]; then
        OPENSSL_PREFIX="/opt/homebrew/opt/openssl@3"
    fi
    if [ -z "$OPENSSL_PREFIX" ] && [ -d "/usr/local/opt/openssl@3" ]; then
        OPENSSL_PREFIX="/usr/local/opt/openssl@3"
    fi

    if [ -n "$OPENSSL_PREFIX" ] && [ -f "$OPENSSL_PREFIX/include/openssl/ssl.h" ]; then
        append_flag_once NGX_CFLAGS "-I${OPENSSL_PREFIX}/include"
        append_flag_once NGX_LDFLAGS "-L${OPENSSL_PREFIX}/lib"
    fi

    PCRE_PREFIX="${PCRE_PREFIX:-}"
    if [ -z "$PCRE_PREFIX" ] && command -v brew >/dev/null 2>&1; then
        PCRE_PREFIX="$(brew --prefix pcre 2>/dev/null || true)"
    fi
    if [ -z "$PCRE_PREFIX" ] && [ -d "/opt/homebrew/opt/pcre" ]; then
        PCRE_PREFIX="/opt/homebrew/opt/pcre"
    fi
    if [ -z "$PCRE_PREFIX" ] && [ -d "/usr/local/opt/pcre" ]; then
        PCRE_PREFIX="/usr/local/opt/pcre"
    fi

    if [ -n "$PCRE_PREFIX" ] && [ -f "$PCRE_PREFIX/include/pcre.h" ]; then
        append_flag_once NGX_CFLAGS "-I${PCRE_PREFIX}/include"
        append_flag_once NGX_LDFLAGS "-L${PCRE_PREFIX}/lib"
    fi
fi

if [ -z "${NGX_VERSION:-}" ]; then
    if command -v nginx >/dev/null 2>&1; then
        NGX_VERSION="$(nginx -v 2>&1 | sed -n 's#.*nginx/\([0-9][0-9.]*\).*#\1#p' | head -n1)"
        if [ -n "$NGX_VERSION" ]; then
            echo "Detected nginx version: $NGX_VERSION"
        else
            echo "Warning: Could not parse nginx version, using default ${DEFAULT_NGX_VERSION}"
            NGX_VERSION="${DEFAULT_NGX_VERSION}"
        fi
    else
        echo "Warning: nginx not found in PATH, using default version ${DEFAULT_NGX_VERSION}"
        NGX_VERSION="${DEFAULT_NGX_VERSION}"
    fi
fi

append_flag_once NGX_CFLAGS "-Wno-deprecated-declarations"

export NGX_VERSION
echo "Building for nginx version: $NGX_VERSION"
cargo build --release -p ngx-args-filter-module

OUTPUT_DIR="${CARGO_TARGET_DIR:-target}"
MODULE_EXT="so"
if [ "$(uname -s)" = "Darwin" ]; then
    MODULE_EXT="dylib"
fi
echo "Module built at: ${OUTPUT_DIR}/release/libngx_args_filter_module.${MODULE_EXT}"
