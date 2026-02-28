# quasiquodo-ts-macros

This crate is a shim for the [`quasiquodo_ts::ts_quote!` macro](https://docs.rs/quasiquodo-ts/latest/quasiquodo_ts/macro.ts_quote.html). The macro is exported from [**quasiquodo-ts**](https://crates.io/crates/quasiquodo-ts); this shim is a separate crate because Rust requires proc macro crates to be their own compilation unit.

**Please use quasiquodo-ts directly.** This crate's API is not stable, and may change without notice.
