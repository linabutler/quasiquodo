# quasiquodo-py

**Compile-time Python quasi-quoting for Rust.**

[<img src="https://img.shields.io/github/v/release/linabutler/quasiquodo?style=for-the-badge&label=tag&logo=rust" alt="GitHub release tag" height="24">](https://github.com/linabutler/quasiquodo/tree/main/quasiquodo-py)
[<img src="https://img.shields.io/github/actions/workflow/status/linabutler/quasiquodo/test.yml?style=for-the-badge&logo=github" alt="Build status" height="24">](https://github.com/linabutler/quasiquodo/actions?query=branch%3Amain)

**quasiquodo-py** is a Rust macro that parses Python strings into typed syntax tree nodes at compile time.

## Getting Started

Add **quasiquodo-py** to your `Cargo.toml`:

```toml
[dependencies]
quasiquodo-py = { git = "https://github.com/linabutler/quasiquodo" }
```

**quasiquodo-py** uses [Ruff](https://github.com/astral-sh/ruff) to parse Python, and re-exports its syntax tree types.

Ruff isn't published to crates.io, so **quasiquodo-py** must be added as a Git dependency. Any Ruff crates you use directly must be pinned to the same Ruff tag that **quasiquodo-py** uses.

Quasiquodo releases are immutable, so it's safe (and encouraged!) to pin **quasiquodo-py** to a `tag` instead of a `rev`.

### Minimum supported Rust version

**quasiquodo-py**'s minimum supported Rust version (MSRV) is **Rust 1.91.0**. The MSRV may increase in minor releases.

## Usage

### Basic quoting

`py_quote!` takes a Python source string and an output kind, and returns the corresponding `ruff_python_ast` node:

```rust
use quasiquodo_py::py_quote;
use quasiquodo_py::ruff::python_ast::*;

let ast = py_quote!("x + 1" as Expr);
```

The output kind—`Expr`, `Stmt`, `FunctionDef`, and so on—tells `py_quote!` how to parse the source and what type to return. The full list of supported constructs appears in the [Reference](#reference).

```rust
let expr = py_quote!("foo(1, 2, 3)" as Expr);

let stmt = py_quote!("return x" as Stmt);

let func = py_quote!(
    "def greet(name: str) -> str:
         return name
    " as FunctionDef
);
```

### Variable substitution

`#{var}` placeholders splice variables into the syntax tree.

```rust
let arg = py_quote!("42" as Expr);
let expr = py_quote!("foo(#{x})" as Expr, x: Expr = arg);
// => `foo(42)`
```

Placeholders can be used in any position:

```rust
let name = Identifier::new("my_func", TextRange::default());
let expr = py_quote!("#{name}()" as Expr, name: Identifier = name);
// => `my_func()`
```

`&str`, `String`, `Box<str>`, `f64`, `u8`, `u16`, `u32`, `u64`, and `bool` variables produce literal nodes:

```rust
let greeting = "hello";
let count = 42.0;
let verbose = true;

let expr = py_quote!(
    "greet(#{g}, #{n}, #{v})" as Expr,
    g: &str = greeting,
    n: f64 = count,
    v: bool = verbose,
);
// => `greet('hello', 42.0, True)`
```

### Splicing

`Vec<T>` variables splice into list positions:

- Function and class bodies
- `for`, `while`, `if`, `with`, and `try` block bodies
- Call expression arguments
- List, set, and tuple elements
- Function parameter lists

```rust
let body = vec![
    py_quote!("y = x + 1" as Stmt),
    py_quote!("return y" as Stmt),
];

let func = py_quote!(
    "def foo(x):
         #{body}
    " as FunctionDef,
    body: Vec<Stmt> = body,
);
```

This produces:

```python
def foo(x):
    y = x + 1
    return y
```

`Option<T>` conditionally includes a value:

```rust
let extra = if include_return {
    Some(py_quote!("return y" as Stmt))
} else {
    None
};

let func = py_quote!(
    "def foo(x):
         y = x + 1
         #{extra}
    " as FunctionDef,
    extra: Option<Stmt> = extra,
);
```

### Docstrings

String variables in statement position become string literal statements, which Python treats as docstrings:

```rust
let doc = "Compute the result.";

let func = py_quote!(
    "def compute(x):
         #{doc}
         return x + 1
    " as FunctionDef,
    doc: &str = doc,
);
```

This produces:

```python
def compute(x):
    'Compute the result.'
    return x + 1
```

Placeholders can also be interpolated into docstring text:

```rust
let noun = "name";
let adj = "required";

let func = py_quote!(
    r#"def foo():
         """The #{noun} is #{adj}."""
         pass
    "# as FunctionDef,
    noun: &str = noun,
    adj: &str = adj,
);
```

This produces:

```python
def foo():
    'The name is required.'
    pass
```

## Reference

### Output kinds

The output kind tells `py_quote!` which `ruff_python_ast` type to return.

| Output kind | AST type | Example source |
|-------------|----------|----------------|
| `Expr` | `Expr` | `"x + 1"` |
| `Stmt` | `Stmt` | `"return x"` |
| `Identifier` | `Identifier` | `"my_var"` |
| `Parameter` | `Parameter` | `"x: int"` |
| `ParameterWithDefault` | `ParameterWithDefault` | `"x=42"` |
| `Decorator` | `Decorator` | `"staticmethod"` |
| `Keyword` | `Keyword` | `"name=\"value\""` |
| `Alias` | `Alias` | `"path as p"` |
| `FunctionDef` | `StmtFunctionDef` | `"def foo(): pass"` |
| `ClassDef` | `StmtClassDef` | `"class Foo: pass"` |
| `ImportFrom` | `StmtImportFrom` | `"from os import path"` |

### Variable types

Variables can be scalar, boxed, or container types.

**Scalar types** substitute a single node or literal value:

| Variable type | Rust value type | Description |
|---------------|-----------------|-------------|
| `Expr` | `Expr` | An expression |
| `Stmt` | `Stmt` | A statement |
| `Identifier` | `Identifier` | An identifier |
| `Parameter` | `Parameter` | A function parameter |
| `ParameterWithDefault` | `ParameterWithDefault` | A parameter with a default value |
| `Decorator` | `Decorator` | A decorator |
| `Keyword` | `Keyword` | A keyword argument |
| `Alias` | `Alias` | An import alias |
| `&str` | `&str` | A string slice, in literal position |
| `String` | `String` | An owned string, in literal position |
| `Box<str>` | `Box<str>` | A boxed string, in literal position |
| `f64` | `f64` | A floating-point number, in literal position |
| `u8` | `u8` | An 8-bit integer, in literal position |
| `u16` | `u16` | A 16-bit integer, in literal position |
| `u32` | `u32` | A 32-bit integer, in literal position |
| `u64` | `u64` | A 64-bit integer, in literal position |
| `bool` | `bool` | A Boolean, in literal position |

**Container types** wrap any scalar type:

| Container | Behavior |
|-----------|----------|
| `Box<T>` | A boxed scalar |
| `Vec<T>` | Splices zero or more items into a list position |
| `Option<T>` | Conditionally splices one item or nothing |
