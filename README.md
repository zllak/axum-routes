# axum-routes

`axum-routes` is a crate on top of [axum](https://github.com/tokio-rs/axum) to
declare routers through enums, and resolve easily routes, so we don't have to
hardcode routes when linking in web apps.

[![Crates.io](https://img.shields.io/crates/v/axum-routes)](https://crates.io/crates/axum-routes)
[![Documentation](https://docs.rs/axum-routes/badge.svg)](https://docs.rs/axum-routes)

## Features
- Declare your `axum::Router` using enums
- Customize routes/nested routers (layers, with_state, fallback, ...)
- Resolve links using the enum, removing the need to hardcode

## Contributing

See a bug ? An improvement ? A new feature you want ? Feel free to open an issue,
or even a PR.

This project is not affiliated with `axum` at all.
