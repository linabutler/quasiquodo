# quasiquodo-py-macros

This crate is a shim for the `quasiquodo_py::py_quote!` macro. The macro is exported from [**quasiquodo-py**](https://github.com/linabutler/quasiquodo/tree/main/quasiquodo-py); this shim is a separate crate because Rust requires proc macro crates to be their own compilation unit.

**Please use quasiquodo-py directly.** This crate's API is not stable, and may change without notice.
