fn main() {
    println!("cargo::rerun-if-env-changed=DEP_NGINX_FEATURES_CHECK");
    println!(
        "cargo::rustc-check-cfg=cfg(ngx_feature, values({}))",
        std::env::var("DEP_NGINX_FEATURES_CHECK").unwrap_or_else(|_| "any()".to_string())
    );

    println!("cargo::rerun-if-env-changed=DEP_NGINX_FEATURES");
    if let Ok(features) = std::env::var("DEP_NGINX_FEATURES") {
        for feature in features.split(',').map(str::trim) {
            println!("cargo::rustc-cfg=ngx_feature=\"{feature}\"");
        }
    }

    // `nginx-sys` exposes the NGINX module ABI version expected by the built binary.
    println!("cargo::rerun-if-env-changed=DEP_NGINX_VERSION_NUMBER");
    let version_number =
        std::env::var("DEP_NGINX_VERSION_NUMBER").unwrap_or_else(|_| "1028001".to_string());

    println!("cargo::rustc-env=NGX_VERSION_NUMBER={version_number}");

    if cfg!(target_os = "macos") {
        println!("cargo::rustc-link-arg=-undefined");
        println!("cargo::rustc-link-arg=dynamic_lookup");
    }
}
