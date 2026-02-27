#![allow(dead_code)]

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use tempfile::TempDir;

struct BuildArtifacts {
    module_path: PathBuf,
    nginx_bin: PathBuf,
}

pub struct NginxTestInstance {
    root: TempDir,
    nginx_bin: PathBuf,
    main_conf: PathBuf,
    pub port: u16,
    base_http_conf: String,
}

impl Drop for NginxTestInstance {
    fn drop(&mut self) {
        let _ = Command::new(&self.nginx_bin)
            .args([
                "-p",
                self.root.path().to_string_lossy().as_ref(),
                "-c",
                self.main_conf.to_string_lossy().as_ref(),
                "-s",
                "stop",
            ])
            .output();
    }
}

fn target_dir() -> PathBuf {
    let workspace_root = find_workspace_root();
    std::env::var("CARGO_TARGET_DIR").map_or_else(|_| workspace_root.join("target"), |raw| {
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            workspace_root.join(p)
        }
    })
}

fn find_workspace_root() -> PathBuf {
    let current_dir = std::env::current_dir().expect("current dir available");
    current_dir
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() && p.join("crates").exists())
        .expect("workspace root should be discoverable from integration-tests crate")
        .to_path_buf()
}

fn integration_tests_dir() -> PathBuf {
    let dir = target_dir().join("integration-tests");
    std::fs::create_dir_all(&dir).expect("create integration-tests dir");
    dir
}

fn runtime_dir() -> PathBuf {
    let dir = integration_tests_dir().join("runtime");
    std::fs::create_dir_all(&dir).expect("create integration runtime dir");
    dir
}

pub fn known_log_path(name: &str) -> PathBuf {
    let file_name = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let path = integration_tests_dir().join("logs");
    std::fs::create_dir_all(&path).expect("create integration logs dir");
    path.join(format!("{file_name}.log"))
}

pub fn unique_log_path(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let thread_id = format!("{:?}", std::thread::current().id())
        .replace("ThreadId(", "")
        .replace(')', "");
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);

    known_log_path(&format!(
        "{prefix}-pid{}-tid{}-{}-{}",
        std::process::id(),
        thread_id,
        now_ns,
        seq
    ))
}

fn artifacts() -> &'static BuildArtifacts {
    static ARTIFACTS: OnceLock<BuildArtifacts> = OnceLock::new();
    ARTIFACTS.get_or_init(|| {
        let build = Command::new("cargo")
            .args(["build", "--release", "-p", "ngx-args-filter-module"])
            .output()
            .expect("cargo build should execute");
        assert!(
            build.status.success(),
            "failed to build module:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        let module_ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };
        let module_path = target_dir()
            .join("release")
            .join(format!("libngx_args_filter_module.{module_ext}"));
        assert!(
            module_path.exists(),
            "module artifact not found at {}",
            module_path.display()
        );

        let nginx_root = target_dir().join("release").join("build");
        let mut stack = vec![nginx_root];
        let mut candidates: Vec<PathBuf> = Vec::new();
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.file_name().is_some_and(|n| n == "nginx") {
                        candidates.push(path);
                    }
                }
            }
        }

        let nginx_bin = candidates
            .into_iter()
            .max_by_key(|p| {
                std::fs::metadata(p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            })
            .expect("vendored nginx binary not found under target/release/build");

        BuildArtifacts {
            module_path,
            nginx_bin,
        }
    })
}

fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind localhost");
    listener.local_addr().expect("local addr").port()
}

fn render_main_conf(
    module_path: &Path,
    root: &Path,
    port: u16,
    http_conf: &str,
    error_log_path: &Path,
) -> String {
    let tmp = root.join("tmp");
    let pid = root.join("nginx.pid");
    format!(
        "load_module {};
worker_processes 1;
error_log {} info;
pid {};
events {{ worker_connections 1024; }}
http {{
  access_log off;
  client_body_temp_path {}/client_body;
  proxy_temp_path {}/proxy;
  fastcgi_temp_path {}/fastcgi;
  uwsgi_temp_path {}/uwsgi;
  scgi_temp_path {}/scgi;
{}
}}
",
        module_path.display(),
        error_log_path.display(),
        pid.display(),
        tmp.display(),
        tmp.display(),
        tmp.display(),
        tmp.display(),
        tmp.display(),
        http_conf.replace("listen 8080", &format!("listen {port}")),
    )
}

fn wait_ready(port: u16) {
    for _ in 0..100 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("nginx did not become ready on port {port}");
}

pub fn setup_nginx(nginx_conf: &str) -> NginxTestInstance {
    let artifacts = artifacts();
    let root = tempfile::Builder::new()
        .prefix("ngxaf-it-")
        .tempdir_in(runtime_dir())
        .expect("temp dir");
    let port = free_port();

    let tmp = root.path().join("tmp");
    std::fs::create_dir_all(root.path().join("logs")).expect("create logs dir");
    std::fs::create_dir_all(tmp.join("client_body")).expect("create client_body");
    std::fs::create_dir_all(tmp.join("proxy")).expect("create proxy");
    std::fs::create_dir_all(tmp.join("fastcgi")).expect("create fastcgi");
    std::fs::create_dir_all(tmp.join("uwsgi")).expect("create uwsgi");
    std::fs::create_dir_all(tmp.join("scgi")).expect("create scgi");

    let main_conf = root.path().join("nginx.conf");
    let error_log = root.path().join("error.log");
    let rendered = render_main_conf(
        &artifacts.module_path,
        root.path(),
        port,
        nginx_conf,
        &error_log,
    );
    std::fs::write(&main_conf, rendered).expect("write nginx conf");

    let test_out = Command::new(&artifacts.nginx_bin)
        .args([
            "-p",
            root.path().to_string_lossy().as_ref(),
            "-c",
            main_conf.to_string_lossy().as_ref(),
            "-t",
        ])
        .output()
        .expect("nginx -t should execute");
    assert!(
        test_out.status.success(),
        "nginx -t failed:\n{}\n{}",
        String::from_utf8_lossy(&test_out.stdout),
        String::from_utf8_lossy(&test_out.stderr)
    );

    let run_out = Command::new(&artifacts.nginx_bin)
        .args([
            "-p",
            root.path().to_string_lossy().as_ref(),
            "-c",
            main_conf.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("nginx start should execute");
    assert!(
        run_out.status.success(),
        "nginx start failed:\n{}\n{}",
        String::from_utf8_lossy(&run_out.stdout),
        String::from_utf8_lossy(&run_out.stderr)
    );

    wait_ready(port);

    NginxTestInstance {
        root,
        nginx_bin: artifacts.nginx_bin.clone(),
        main_conf,
        port,
        base_http_conf: nginx_conf.to_string(),
    }
}

pub async fn send_request(
    nginx: &NginxTestInstance,
    path: &str,
    query: Option<&str>,
) -> reqwest::Response {
    let url = query.map_or_else(
        || format!("http://127.0.0.1:{}{path}", nginx.port),
        |q| format!("http://127.0.0.1:{}{path}?{q}", nginx.port),
    );
    reqwest::get(&url).await.expect("Failed to send request")
}

pub fn run_nginx_config_test(
    nginx: &NginxTestInstance,
    _conf_name: &str,
    conf_content: &str,
) -> (String, String) {
    let artifacts = artifacts();
    let test_conf = nginx.root.path().join("nginx-test.conf");
    let test_err = nginx.root.path().join("nginx-test-error.log");
    let merged = format!("{}\n{}", nginx.base_http_conf, conf_content);
    let rendered = render_main_conf(
        &artifacts.module_path,
        nginx.root.path(),
        nginx.port,
        &merged,
        &test_err,
    );
    std::fs::write(&test_conf, rendered).expect("write test nginx conf");

    let output = Command::new(&artifacts.nginx_bin)
        .args([
            "-p",
            nginx.root.path().to_string_lossy().as_ref(),
            "-c",
            test_conf.to_string_lossy().as_ref(),
            "-t",
        ])
        .output()
        .expect("nginx -t test command should execute");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

pub fn truncate_error_log(path: &Path) {
    let _ = std::fs::write(path, "");
}

pub fn read_error_log(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}
