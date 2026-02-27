mod helpers;

struct Case {
    name: &'static str,
    conf: &'static str,
    expected_stderr: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "args_filter_variable_must_start_with_dollar",
        conf: r"
args_filter filtered_args {
    initial all;
}
",
        expected_stderr: "args_filter variable must start with '$'",
    },
    Case {
        name: "args_filter_variable_name_cannot_be_empty",
        conf: r"
args_filter $ {
    initial all;
}
",
        expected_stderr: "args_filter variable name cannot be empty",
    },
    Case {
        name: "args_filter_variable_name_invalid_characters",
        conf: r"
args_filter $filtered-args {
    initial all;
}
",
        expected_stderr: "args_filter variable name contains invalid characters",
    },
    Case {
        name: "initial_directive_duplicate",
        conf: r"
args_filter $dup_initial {
    initial all;
    initial none;
}
",
        expected_stderr: "directive is duplicate",
    },
    Case {
        name: "initial_value_must_be_all_or_none",
        conf: r"
args_filter $bad_initial {
    initial maybe;
}
",
        expected_stderr: "must be \"all\" or \"none\"",
    },
    Case {
        name: "include_mode_must_be_regex_operator_when_three_args",
        conf: r#"
args_filter $bad_include_mode {
    initial all;
    include equals "^x$";
}
"#,
        expected_stderr: "expects literal, \"~\", or \"~*\"",
    },
    Case {
        name: "exclude_mode_must_be_regex_operator_when_three_args",
        conf: r#"
args_filter $bad_exclude_mode {
    initial all;
    exclude equals "^x$";
}
"#,
        expected_stderr: "expects literal, \"~\", or \"~*\"",
    },
    Case {
        name: "exclude_regex_must_compile",
        conf: r#"
args_filter $bad_exclude_regex {
    initial all;
    exclude ~ "(";
}
"#,
        expected_stderr: "failed to compile regex:",
    },
    Case {
        name: "unknown_nested_directive_is_rejected",
        conf: r"
args_filter $unknown_nested {
    initial all;
    unknown_rule x;
}
",
        expected_stderr: "unknown directive inside args_filter block",
    },
    Case {
        name: "initial_wrong_arity_rejected_by_nginx_parser",
        conf: r"
args_filter $bad_initial_arity {
    initial;
}
",
        expected_stderr: "invalid number of arguments in \"initial\" directive",
    },
    Case {
        name: "include_wrong_arity_rejected_by_nginx_parser",
        conf: r"
args_filter $bad_include_arity {
    initial all;
    include;
}
",
        expected_stderr: "invalid number of arguments in \"include\" directive",
    },
    Case {
        name: "exclude_wrong_arity_rejected_by_nginx_parser",
        conf: r"
args_filter $bad_exclude_arity {
    initial all;
    exclude;
}
",
        expected_stderr: "invalid number of arguments in \"exclude\" directive",
    },
    Case {
        name: "volatile_wrong_arity_rejected",
        conf: r"
args_filter $bad_volatile_arity {
    initial all;
    volatile on;
}
",
        expected_stderr: "invalid number of arguments in \"volatile\" directive",
    },
];

const NGINX_CONF: &str = r#"
server {
    listen 8080 default_server;
    server_name _;

    location / {
        default_type text/plain;
        return 200 "ok";
    }
}
"#;

fn assert_nginx_t_failed(output: &str, case_name: &str) {
    assert!(
        !output.contains("test is successful"),
        "expected nginx -t to fail for `{case_name}`, but nginx reported success:\n{output}"
    );
}

fn assert_error_fragment(output: &str, case_name: &str, expected: &str) {
    if output.contains(expected) {
        return;
    }

    // NGINX may drop quote characters when formatting custom error logs.
    let normalized_stderr = output.replace('"', "");
    let normalized_expected = expected.replace('"', "");
    assert!(
        normalized_stderr.contains(&normalized_expected),
        "expected nginx -t output to contain `{expected}` for `{case_name}`. output was:\n{output}"
    );
}

#[tokio::test]
async fn test_args_filter_config_validation_errors_matrix() {
    let nginx = helpers::setup_nginx(NGINX_CONF);

    for case in CASES {
        let (stdout, stderr) =
            helpers::run_nginx_config_test(&nginx, case.name, case.conf);
        let output = format!("{stdout}\n{stderr}");
        println!("output: {output}");
        assert_nginx_t_failed(&output, case.name);
        assert_error_fragment(&output, case.name, case.expected_stderr);
    }
}
