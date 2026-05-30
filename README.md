# srvcs-symmetricdifference

The symmetric-difference service of the srvcs.cloud distributed standard library.

Its single concern: **the symmetric difference of two sets of integers.** It
reads two lists `a` and `b` and returns the sorted list of distinct values that
appear in exactly one of the two lists (but not both).

`srvcs-symmetricdifference` is a **leaf**: it depends on no other service and
makes no network calls. All work is local.

```text
result = sorted distinct values in exactly one of a or b
symmetricdifference([1, 2, 3], [2, 3, 4]) == [1, 4]
```

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Symmetric difference of `a` and `b` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"a": [1, 2, 3], "b": [2, 3, 4]}'
# {"a":[1,2,3],"b":[2,3,4],"result":[1,4]}

curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"a": [1, 1, 2], "b": [2, 2]}'
# {"a":[1,1,2],"b":[2,2],"result":[1]}
```

Responses:

- `200 {"a": [...], "b": [...], "result": [...]}` — evaluated. `result` is the
  sorted list of distinct values appearing in exactly one of `a` or `b`.
- `422 {"error": "a and b must be lists of integers"}` — some element of `a` or
  `b` is not a JSON integer.

The result is always sorted ascending and contains distinct values. Duplicates
within a list are collapsed (the lists are treated as sets). The symmetric
difference of two empty (or identical) sets is the empty list. Negatives are
ordered correctly.

## Dependencies

None. `srvcs-symmetricdifference` is a leaf set service. Because it owns its own
validation, it rejects any non-integer element directly with `422` rather than
forwarding to a dependency.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared
standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
