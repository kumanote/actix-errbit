# actix-errbit

This crate adds a middleware for actix-web that captures errors and report them to errbit.

**Notes**

- This crate supports [anyhow](https://github.com/dtolnay/anyhow) Error type for reporting with backtrace.
- There will be no backtrace output for `std::error::Error`.

## Installation

#### Dependencies

- [Rust with Cargo](http://rust-lang.org)

**rust-toolchain**

```text
1.54.0
```

#### Importing

**~/.cargo/config**

```toml
[net]
git-fetch-with-cli = true
```

**Cargo.toml**

```toml
[dependencies]
actix-errbit = { version = "0.1.0", git = "ssh://git@github.com/kumanote/actix-errbit.git", branch = "main" }
```

## Configurations

You can set your default `host`/`project id`/`project key`/`environment` values by setting the following environment
variables.

| ENV VAR NAME | CONTENT | EXAMPLE |
| --- | --- | --- |
| `AIRBRAKE_HOST` | your errbit server host name | `https://api.airbrake.io` |
| `AIRBRAKE_PROJECT_ID` | your errbit project id | `1` |
| `AIRBRAKE_PROJECT_ID` | your errbit project api key | `ffcbf68d38782ae9ba32591a859f1452` |
| `AIRBRAKE_ENVIRONMENT` | your application environment | `development` / `dev` / `staging` |



## Examples

Here's a basic example:

```rust
use actix_errbit::{Config as ErrbitConfig, Errbit};
use actix_web::{web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut errbit_config = ErrbitConfig::default();
    errbit_config.host = "https://errbit.yourdomain.com".to_owned();
    errbit_config.project_id = "1".to_owned();
    errbit_config.project_key = "ffffffffffffffffffffffffffffffff".to_owned();
    errbit_config.environment = Some("staging".to_owned());
    HttpServer::new(move || {
        let errbit = Errbit::new(errbit_config.clone())
            .expect("the errbit endpoint to report error to must be configured...");
        App::new()
            .wrap(errbit)
            .service(web::resource("/").to(|| async { "Hello world!" }))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
```
