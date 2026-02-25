# AGENTS.md

Guidance for AI coding agents. Follow exactly; overrides defaults. `CLAUDE.md` is a symlink.

---

## Verification Checklist

After making changes, **always** run in order:

```bash
cargo check --workspace
cargo test --workspace --no-fail-fast --all-features
cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged --no-deps # Auto-fixes lint suggestions
cargo +nightly fmt --all
```

**Task is not complete until all commands pass.** If any fails: fix, re-run from step 1, repeat.

**If failing 3+ times:** Stop, re-read errors carefully, check if failure is in your code or pre-existing, ask for guidance if stuck.

---

## Architecture

Compile-time TypeScript quasiquoting library. Parses TypeScript source literals at compile time via SWC, then generates Rust code that constructs equivalent `swc_ecma_ast` AST nodes:

```rust
ts_quote!("string | null" as TsType)
ts_quote!("foo(#{arg})" as Expr, arg: Expr = my_expr)
```

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| **quasiquodo** | Public API: re-exports macros and `swc_ecma_utils`; integration tests |
| **quasiquodo-ts-core** | Core logic: input parsing, preprocessing, SWC parsing, AST-to-Rust codegen |
| **quasiquodo-ts-macros** | Thin proc-macro shim that delegates to `quasiquodo-ts-core` |

### Key Abstractions

**`OutputKind`** (`input.rs`) — Determines how the TypeScript source is parsed and what AST type is extracted. Each variant wraps the source in valid syntax for SWC (e.g. `TsType` wraps as `type __T = <source>;`). See the enum for the full list of variants.

**`VarType`** (`input.rs`) — The type of a substitution variable. Scalar types mirror `OutputKind` plus literal types (`&str`, `String`, `f64`, `usize`, `bool`), with nestable containers (`Box<T>`, `Vec<T>`, `Option<T>`). `Vec<T>` and `Option<T>` variables splice into list positions. String types use `Str(StrVarType)` and numeric types use `Num(NumVarType)` inner enums.

**Two-phase variable substitution** — Phase 1 (lexer/preprocessing): `#{var}` placeholders in the source string are replaced with type-appropriate stand-ins before SWC parses (`Str(_)` → `"__tsq_N__"`, others → `__tsq_N__`). Phase 2 (codegen): the `Lift` trait detects stand-ins in the parsed AST and injects the variable's Rust expression.

**`Lift` trait** (`lift/mod.rs`) — Converts SWC AST nodes into `syn::Expr` that constructs them at runtime. ~100 implementations covering all SWC node types, split across submodules (`primitives`, `expressions`, `statements`, `declarations`, `types`, `modules`). Uses `impl_lift_for_struct!` and `impl_lift_for_enum!` macros for boilerplate.

**`CodeFragment`** (`lift/mod.rs`) — Either `Single(expr)` (a single AST node constructor) or `Splice(expr)` (an iterator for `Vec`/`Option` variables). Splicing bubbles up through containers until caught by a `Vec<T>` position; reaching a non-iterable position is an error.

**`Context`** (`context.rs`) — Manages variable bindings during codegen. Handles span resolution (custom span or `DUMMY_SP`), JSDoc comment extraction from SWC's comment map, and variable lookup by stand-in.

---

## Coding Style

These are requirements, not suggestions. Violations will produce unacceptable code. When rules conflict: consistency wins, more specific rules apply, ask if genuinely unclear.

### Type Design

- **Context objects:** Bundle related data in structs instead of free functions with many params.
- **Newtypes:** Use to enforce invariants.
- **Enums with data:** Carry data in variants directly.
- **Symmetry:** Similar types follow similar patterns, even if slightly redundant.

### Ownership and Lifetimes

```rust
// ✅ Borrow from source
struct MyView<'a> {
    name: &'a str,
    items: &'a [Item],
}

// ❌ Unnecessary allocation
struct MyView {
    name: String,
    items: Vec<Item>,
}
```

- Use semantic names (`'view` for views, `'graph` for graphs) when multiple lifetimes coexist; `'a` is fine for single-lifetime cases.
- Never elide lifetimes that distinguish borrowed sources.
- **Deref coercion:** Use `&*vec` for `&[T]`, `&*r` for `&T`, `&**ref_to_box` for `&T` from `&Box<T>`.

### Documentation (`///`)

- Complete sentences with periods
- **Backtick code items.** Anything that appears verbatim in source code: types (`Expr`, `JsDoc`), functions (`lift()`), fields (`span`), macros (`ts_quote!`), syntax (`/*`, `#{}`), literals (`None`), paths (`crate::lift`). Not general concepts (JSDoc, quasiquoting, placeholder) or project names used as nouns (SWC).
- Indicative mood ("Returns", not "Return")
- Describe args/returns in prose, never separate sections
- Wrap at 80 chars

```rust
// ✅ Indicative mood, inline prose, complete sentences, backticks
/// Creates and returns a representation of a feature-gated `impl Client` block
/// for a resource, with all its operations.
pub fn new(resource: &'a str, operations: &'a [IrOperationView<'a>]) -> Self { ... }

// ❌ Imperative mood, separate sections, sentence fragments, no backticks
/// Create a representation of a feature-gated impl Client block
///
/// # Arguments
/// - resource (string): The resource name
/// - operations (list): The operations
///
/// # Returns
/// The representation
pub fn new(resource: &'a str, operations: &'a [IrOperationView<'a>]) -> Self { ... }
```

### Comments (`//`)

- Only for non-obvious logic
- Backtick code items (same rules as `///` docs above)
- `//` comments: wrap at 80 chars; complete sentences with periods
- `// MARK:` for sections: ~50 chars; no period

```rust
// ✅ Explains why, complete sentence, backticks
// Skip `f.discriminator`; it's handled separately in tagged unions.
if f.discriminator() { continue; }

// ❌ Restates code, sentence fragment, no backticks around `f`
// Check if f is discriminator
if f.discriminator() { continue; }
```

### Strings

- Raw strings (`r#"..."#`) for strings with quotes; never `\"`
- `.to_owned()` for `&str` → `String`
- `.to_string()` only when formatting (numbers, `Display` types)

### Other

- **Imports:** Ordered groups (blank lines between): `std::` → external crates (alphabetical) → `crate::` → `super::`. No globs except re-exports in `mod.rs`, `use super::*` in tests.
- Justify lint suppressions with comments

---

## Testing

- **Naming:** `test_<behavior>_<condition>`, grouped with `// MARK:` comments.
- **No new helper functions.** Inline all fixtures directly. Use existing helpers, don't add new ones without asking.
- **Use `indoc::indoc!` for multi-line strings,** never `\n`.
- **Assertions:** Include actual value in `let-else` panic messages: `panic!("expected `X`; got `{ty:?}`")`.
- **Throwaway tests:** When behavior is unclear, write a quick test to prove it rather than theorizing. Delete once answered, or promote to a permanent test.
- **Debugging `syn` node mismatches**: When `assert_eq!(actual, expected)` fails, use `println!("{}", actual.to_token_stream())` to compare expectations.

---

## Crate-Specific Rules

### quasiquodo-ts-core

- **`lift/` is the largest module.** Use `impl_lift_for_struct!` and `impl_lift_for_enum!` macros for new `Lift` implementations rather than hand-writing them. Follow the field-mapping pattern established by existing impls. Place new impls in the appropriate submodule (`primitives`, `expressions`, `statements`, `declarations`, `types`, `modules`).
- **Adding a new `OutputKind`:** Add variant to enum, add wrapping/extraction logic in `expand()`, add parsing arm in `OutputKind::parse()`. Follow existing patterns — each variant documents how source is wrapped for SWC parsing.
- **Adding a new `VarType`:** Add variant to enum, add parsing arm, add placeholder logic in `lexer.rs`, add substitution handling in `lift/`. Ensure `Vec<NewType>` splicing works in appropriate list positions.
- **Preprocessing invariant:** After `preprocess()`, the source must be valid TypeScript that SWC can parse. Stand-ins must be syntactically valid in their position (identifiers or string literals).

### quasiquodo

- **Integration tests only.** No library logic lives here — it re-exports from the other crates.
- **Test pattern:** Each test file covers one `OutputKind` or feature. Tests call `ts_quote!`, convert the result to a code string via SWC's `to_code()`, and assert against expected TypeScript output using `indoc::indoc!`.

---

## Process

- **Dependencies:** Use `[workspace.dependencies]` for dependencies shared between workspace crates; package `[dependencies]` for unshared. Justify new deps.
- **Breaking changes:** Make breaking changes; don't prioritize backward-compatibility.
- **Design:** Push back or propose alternatives. Keep changes modular for partial reverts.
- **Ask for help when:** requirements ambiguous, multiple valid approaches, tests fail for unclear reasons, scope larger than expected, new workspace crate needed, or approach seems wrong.
