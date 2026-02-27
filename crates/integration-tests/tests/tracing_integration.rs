mod helpers;

struct LogCleanup(std::path::PathBuf);

impl Drop for LogCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[tokio::test]
async fn test_debug_logs_are_emitted_when_nginx_log_level_is_debug() {
    let test_error_log_path = helpers::unique_log_path("tracing-debug");
    let _cleanup = LogCleanup(test_error_log_path.clone());
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    include keep;
}

server {
    listen 8080 default_server;
    server_name _;
    error_log __TEST_ERROR_LOG_PATH__ debug;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#
    .replace(
        "__TEST_ERROR_LOG_PATH__",
        test_error_log_path.to_string_lossy().as_ref(),
    );

    let nginx = helpers::setup_nginx(&nginx_conf);
    helpers::truncate_error_log(&test_error_log_path);

    let response = helpers::send_request(&nginx, "/", Some("keep=1&drop=2")).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "keep=1");

    let log = helpers::read_error_log(&test_error_log_path);
    assert!(
        log.contains("args_filter: evaluating variable='$filtered_args'"),
        "expected request debug trace in error log, got:\n{log}"
    );
}

#[tokio::test]
async fn test_debug_logs_are_not_emitted_when_nginx_log_level_is_error() {
    let test_error_log_path = helpers::unique_log_path("tracing-error");
    let _cleanup = LogCleanup(test_error_log_path.clone());
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    include keep;
}

server {
    listen 8080 default_server;
    server_name _;
    error_log __TEST_ERROR_LOG_PATH__ error;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#
    .replace(
        "__TEST_ERROR_LOG_PATH__",
        test_error_log_path.to_string_lossy().as_ref(),
    );

    let nginx = helpers::setup_nginx(&nginx_conf);
    helpers::truncate_error_log(&test_error_log_path);

    let response = helpers::send_request(&nginx, "/", Some("keep=1&drop=2")).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "keep=1");

    let log = helpers::read_error_log(&test_error_log_path);
    assert!(
        !log.contains("args_filter: evaluating variable='$filtered_args'"),
        "did not expect request debug trace at error log level, got:\n{log}"
    );
}
