mod helpers;

#[tokio::test]
async fn test_args_filter_initial_all_with_exclude_and_include_override() {
    let nginx_conf = r#"
args_filter $filtered_args {
    initial all;
    exclude ~ "^ads\.";
    include ads.test;
}

server {
    listen 8080 default_server;
    server_name _;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#;

    let nginx = helpers::setup_nginx(nginx_conf);

    let response =
        helpers::send_request(&nginx, "/", Some("x=1&ads.foo=2&ads.test=3&y=4")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "x=1&ads.test=3&y=4");
}

#[tokio::test]
async fn test_args_filter_initial_none_with_include_and_exclude_override() {
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    include ~ "^ads\.";
    include y;
    exclude ads.test;
}

server {
    listen 8080 default_server;
    server_name _;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#;

    let nginx = helpers::setup_nginx(nginx_conf);

    let response =
        helpers::send_request(&nginx, "/", Some("x=1&ads.foo=2&ads.test=3&y=4")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "ads.foo=2&y=4");
}

#[tokio::test]
async fn test_args_filter_initial_none_with_includes() {
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    include a;
    include b;
}

server {
    listen 8080 default_server;
    server_name _;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#;

    let nginx = helpers::setup_nginx(nginx_conf);

    let response = helpers::send_request(&nginx, "/", Some("a=1&c=3&b=2")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "a=1&b=2");
}

#[tokio::test]
async fn test_args_filter_array_style_keys_preserve_all_values_and_order() {
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    include test[];
}

server {
    listen 8080 default_server;
    server_name _;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#;

    let nginx = helpers::setup_nginx(nginx_conf);

    let response =
        helpers::send_request(&nginx, "/", Some("a=0&test[]=1&b=2&test[]=3&test[]=4")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "test[]=1&test[]=3&test[]=4");
}

#[tokio::test]
async fn test_args_filter_mixed_literal_and_regex_keeps_original_order() {
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    include ~ "^test\[\]$";
    include y;
    exclude test[];
    include test[];
}

server {
    listen 8080 default_server;
    server_name _;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#;

    let nginx = helpers::setup_nginx(nginx_conf);

    let response =
        helpers::send_request(&nginx, "/", Some("x=1&test[]=1&y=2&test[]=3&z=4")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "test[]=1&y=2&test[]=3");
}

#[tokio::test]
async fn test_args_filter_supports_volatile_directive() {
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    volatile;
    include a;
}

server {
    listen 8080 default_server;
    server_name _;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#;

    let nginx = helpers::setup_nginx(nginx_conf);

    let response = helpers::send_request(&nginx, "/", Some("a=1&b=2")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "a=1");
}

#[tokio::test]
async fn test_args_filter_volatile_re_evaluates_after_args_mutation() {
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    volatile;
    include keep;
    include modify;
}

server {
    listen 8080 default_server;
    server_name _;

    location / {
        set $some_var $filtered_args;
        set $args "modify=this";
        default_type text/plain;
        return 200 "before:$some_var after:$filtered_args";
    }
}
"#;

    let nginx = helpers::setup_nginx(nginx_conf);

    let response = helpers::send_request(&nginx, "/", Some("keep=1&drop=2")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.text().await.unwrap(),
        "before:keep=1 after:modify=this"
    );
}

#[tokio::test]
async fn test_args_filter_preserves_percent_encoded_plus_in_output() {
    let nginx_conf = r#"
args_filter $filtered_args {
    initial none;
    include keep;
    include keep2;
}

server {
    listen 8080 default_server;
    server_name _;

    location / {
        default_type text/plain;
        return 200 "$filtered_args";
    }
}
"#;

    let nginx = helpers::setup_nginx(nginx_conf);

    let response =
        helpers::send_request(&nginx, "/", Some("keep=%2B&drop=x+y&keep2=a%2Bb")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "keep=%2B&keep2=a%2Bb");
}
