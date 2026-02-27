# Module Directive Reference

## Directive: `args_filter`

Syntax:

```nginx
args_filter $variable_name {
    [initial all | none;]
    include <literal>;
    include ~ <regex>;
    include ~* <regex>;
    exclude <literal>;
    exclude ~ <regex>;
    exclude ~* <regex>;
    volatile;
}
```

Context:

- `http` main context

## Semantics

- `initial` is optional. If omitted, the default is `none`.
- `initial all`: keep all keys unless later excluded.
- `initial none`: drop all keys unless later included.
- Rules are evaluated in declaration order.
- Last matching rule wins.
- Matching uses raw key bytes from query-string segments (no percent-decoding).

## `volatile;`

- No arguments.
- Optional nested directive.
- Without `volatile;`, the variable is cacheable for request evaluation.
- With `volatile;`, nginx sets `no_cacheable = 1`, mirroring `map`-style volatility behavior.

## Example

```nginx
args_filter $filtered_args {
    initial all;
    volatile;
    exclude ~ "^utm_";
    include utm_source;
}

location / {
    return 200 "$filtered_args";
}
```

## Validation Notes

- Variable name must start with `$`.
- Variable name allows only `[A-Za-z0-9_]` after `$`.
- `volatile` with arguments is rejected.
- Invalid regex patterns fail configuration validation (`nginx -t`).

## Runtime Behavior

- Output preserves input segment order for kept keys.
- Repeated keys (for example `test[]=1&test[]=2`) preserve all matching entries in order.
- Empty query string yields an empty variable value.
