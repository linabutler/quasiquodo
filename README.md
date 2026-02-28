# Quasiquodo

**Compile-time quasi-quoting for typed languages.**

[<img src="https://img.shields.io/crates/v/quasiquodo?style=for-the-badge&logo=rust" alt="crates.io" height="24">](https://crates.io/crates/quasiquodo)
[<img src="https://img.shields.io/github/actions/workflow/status/linabutler/quasiquodo/test.yml?style=for-the-badge&logo=github" alt="Build status" height="24">](https://github.com/linabutler/quasiquodo/actions?query=branch%3Amain)
[<img src="https://img.shields.io/docsrs/quasiquodo/latest?style=for-the-badge&logo=docs.rs" alt="Documentation" height="24">](https://docs.rs/quasiquodo)

Quasiquodo is a family of Rust macros that parse TypeScript and Python strings into correct-by-construction syntax trees at compile time.

Instead of building syntax trees by hand:

```rust
let ts = TsType::TsUnionOrIntersectionType(
    TsUnionOrIntersectionType::TsUnionType(TsUnionType {
        span: DUMMY_SP,
        types: vec![
            Box::new(TsType::TsKeywordType(TsKeywordType {
                span: DUMMY_SP,
                kind: TsKeywordTypeKind::TsStringKeyword,
            })),
            Box::new(TsType::TsKeywordType(TsKeywordType {
                span: DUMMY_SP,
                kind: TsKeywordTypeKind::TsNullKeyword,
            })),
        ],
    }),
);

let py = Expr::BinOp(ExprBinOp {
    node_index: AtomicNodeIndex::NONE,
    range: TextRange::default(),
    left: Box::new(Expr::Name(ExprName {
        node_index: AtomicNodeIndex::NONE,
        range: TextRange::default(),
        id: Name::new_static("x"),
        ctx: ExprContext::Load,
    })),
    op: Operator::Add,
    right: Box::new(Expr::NumberLiteral(ExprNumberLiteral {
        node_index: AtomicNodeIndex::NONE,
        range: TextRange::default(),
        value: Number::Int(Int::ONE),
    })),
});
```

...Quasiquodo lets you write:

```rust
let ts = ts_quote!("string | null" as TsType);

let py = py_quote!("x + 1" as Expr);
```

## Getting Started

Quasiquodo provides language-specific crates, each with their own macro and documentation:

* [**quasiquodo-ts**](https://crates.io/crates/quasiquodo-ts): `ts_quote!` for TypeScript
* [**quasiquodo-py**](https://github.com/linabutler/quasiquodo/tree/main/quasiquodo-py): `py_quote!` for Python

The **quasiquodo** umbrella crate re-exports **quasiquodo-ts** behind the `ts` feature flag, enabled by default. **quasiquodo-py** depends on [Ruff](https://github.com/astral-sh/ruff), which isn't published to crates.io, so it must be added as a separate Git dependency.

## How It Works

The Quasiquodo macros expand into pure Rust block expressions—no runtime parsing, just construction code.

When one of these macros runs, it replaces `#{var}` placeholders with syntactically appropriate stand-ins, so that the result is valid source code in the target language. It then parses that source with the language's parser, and extracts the requested output type from the AST. If the source contains invalid syntax, the macro reports the error as a compile-time diagnostic.

Quasiquodo then _unparses_ the AST, turning each node into a Rust expression that constructs the equivalent node in your program. Along the way, it replaces the stand-ins with the bound variables. The result is Rust code that builds the AST directly.

## Contributing

We love contributions!

If you find a case where Quasiquodo fails, generates incorrect output, or lacks an output kind you need, please [open an issue](https://github.com/linabutler/quasiquodo/issues/new) with a minimal reproducing macro invocation.

For questions or larger contributions, please [start a discussion](https://github.com/linabutler/quasiquodo/discussions).

Quasiquodo follows [the Ghostty project's AI Usage policy](https://github.com/ghostty-org/ghostty/blob/1fa4e787eb1f50729153d09b7f455ebb9fc4ccc9/AI_POLICY.md).

## Acknowledgments

Quasiquodo builds on:

* [**Ruff**](https://docs.astral.sh/ruff/), whose parser and AST make **quasiquodo-py** possible.
* [**SWC**](https://swc.rs), whose parser and AST make **quasiquodo-ts** possible, and whose quasi-quotation macro for JavaScript inspired Quasiquodo's design.
