# quasiquodo-ts-macros

This crate is a shim for [Quasiquodo's](https://crates.io/crates/quasiquodo) `ts_quote!` macro. The macro is implemented in `quasiquodo-ts-core`; this shim is a separate crate because Rust requires proc macro crates to be their own compilation unit.

**Please use `quasiquodo` directly.** This crate's API is not stable, and may change without notice.
