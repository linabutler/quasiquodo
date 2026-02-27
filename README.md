# Quasiquodo

**Compile-time TypeScript quasi-quoting for Rust.**

[<img src="https://img.shields.io/crates/v/quasiquodo?style=for-the-badge&logo=rust" alt="crates.io" height="24">](https://crates.io/crates/quasiquodo)
[<img src="https://img.shields.io/github/actions/workflow/status/linabutler/quasiquodo/test.yml?style=for-the-badge&logo=github" alt="Build status" height="24">](https://github.com/linabutler/quasiquodo/actions?query=branch%3Amain)
[<img src="https://img.shields.io/docsrs/quasiquodo/latest?style=for-the-badge&logo=docs.rs" alt="Documentation" height="24">](https://docs.rs/quasiquodo)

Quasiquodo is a Rust macro that turns inline TypeScript into correct-by-construction syntax trees, giving you TypeScript ergonomics with compile-time type safety.

Instead of building a syntax tree by hand:

```rust
let ast = TsType::TsUnionOrIntersectionType(
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
```

...Quasiquodo lets you write:

```rust
let ast = ts_quote!("string | null" as TsType);
```

## Getting Started

Add Quasiquodo to your `Cargo.toml`:

```toml
[dependencies]
quasiquodo = "0.3"
```

Quasiquodo uses [SWC](https://swc.rs) to parse TypeScript, and [re-exports its syntax tree types](https://docs.rs/quasiquodo/latest/quasiquodo/ts/swc/index.html). Any SWC crates you use directly—like `swc_ecma_codegen` for code generation—must be added as separate dependencies.

### Minimum supported Rust version

Quasiquodo's minimum supported Rust version (MSRV) is **Rust 1.89.0**. The MSRV may increase in minor releases (e.g., Quasiquodo 1.1.x may require a newer MSRV than 1.0.x).

## Usage

### Basic quoting

`ts_quote!` takes a TypeScript source string and an output kind, and returns the corresponding [`swc_ecma_ast`](https://rustdoc.swc.rs/swc_ecma_ast/) node:

```rust
use quasiquodo::ts_quote;
use quasiquodo::ts::swc::ecma_ast::*;

let ast = ts_quote!("string | null" as TsType);
```

The output kind—`TsType`, `Expr`, `ModuleItem`, and so on—tells `ts_quote!` how to parse the source and what type to return. You can quote any TypeScript construct that has an output kind:

```rust
let ty = ts_quote!("Record<string, number>" as TsType);

let expr = ts_quote!("foo()" as Expr);

let iface = ts_quote!("interface Pet { name: string; age?: number; }" as Decl);
```

### Variable substitution

`#{var}` placeholders splice variables into the syntax tree. Each variable is declared with a name, type, and value:

```rust
let name = ts_quote!("Pet" as Ident);
let field_type = ts_quote!("string[]" as TsType);

let ast = ts_quote!(
    "#{name}: #{field_type}" as TsTypeElement,
    name: Ident = name,
    field_type: TsType = field_type,
);
```

Placeholders can be used in any position:

```rust
let module = "./types";

let ast = ts_quote!(
    "import type { Pet } from #{module};" as ModuleItem,
    module: &str = module,
);
// => `import type { Pet } from "./types";`
```

Variables accept any value that converts into their declared type, not just the exact type. You can pass inner types without wrapping them in their containing variants, or pass strings as identifiers:

```rust
// A `Decl` can be passed where a `Stmt` is expected.
let type_alias = ts_quote!("type Id = string" as Decl);
let ast = ts_quote!("{ #{s} }" as Stmt, s: Stmt = type_alias);

// An `Ident` variable can be passed as a `String` or `&str`.
let name = "greet";
let ast = ts_quote!("#{name}()" as Expr, name: Ident = name);
// => `greet()`
```

`&str`, `String`, `f64`, `usize`, and `bool` variables become literals:

```rust
let greeting = "hello";
let count = 3.0;
let verbose = true;

let ast = ts_quote!(
    "greet(#{g}, #{n}, #{v})" as Expr,
    g: &str = greeting,
    n: f64 = count,
    v: bool = verbose,
);
// => `greet("hello", 3, true)`
```

In property name and member access positions, string variables simplify to plain identifiers when the value is a valid identifier:

```rust
let name = "color";
let ast = ts_quote!("#{name}: string" as TsTypeElement, name: &str = name);
// => `color: string;`

let name = "background-color";
let ast = ts_quote!("#{name}: string" as TsTypeElement, name: &str = name);
// => `"background-color": string;`

let field = "name";
let ast = ts_quote!("foo[#{f}]" as Expr, f: &str = field);
// => `foo.name`

let field = "some-field";
let ast = ts_quote!("foo[#{f}]" as Expr, f: &str = field);
// => `foo["some-field"]`
```

### Splicing

`Vec<T>` variables splice into list positions:

- Union and intersection type arms
- Interface `extends` clauses
- Interface and class bodies
- Function and constructor parameter lists
- Call expression arguments
- Array literal elements
- Object literal members
- Import and export specifier lists
- Block statement bodies

```rust
let name = ts_quote!("Pet" as Ident);
let members = vec![
    ts_quote!("name: string" as TsTypeElement),
    ts_quote!("age?: number" as TsTypeElement),
];

let ast = ts_quote!(
    "export interface #{name} { #{members}; }" as ModuleItem,
    name: Ident = name,
    members: Vec<TsTypeElement> = members,
);
```

This produces:

```typescript
export interface Pet {
    name: string;
    age?: number;
}
```

Some positions, such as union and intersection types, use `Box<T>` wrapping:

```rust
let extra = vec![
    Box::new(ts_quote!("number" as TsType)),
    Box::new(ts_quote!("boolean" as TsType)),
];

let ast = ts_quote!(
    "string | #{extra}" as TsType,
    extra: Vec<Box<TsType>> = extra,
);
// => `string | number | boolean`
```

`Option<T>` conditionally includes a value:

```rust
let extra = if include_age {
    Some(ts_quote!("age?: number" as TsTypeElement))
} else {
    None
};

let ast = ts_quote!(
    "export interface Pet { name: string; #{extra}; }" as ModuleItem,
    extra: Option<TsTypeElement> = extra,
);
```

### Custom spans

The optional `span` parameter applies a custom [`Span`](https://rustdoc.swc.rs/swc_core/common/struct.Span.html) to every node in the returned tree, useful for pointing diagnostics to the right source location:

```rust
use quasiquodo::ts::swc::common::{BytePos, Span};

let ast = ts_quote!(
    span = Span::new(BytePos(10), BytePos(25)),
    "name: string" as TsTypeElement,
);
```

### JSDoc comments

`ts_quote!` understands JSDoc-style `/** ... */` comments, and supports splicing string variables into them:

```rust
use quasiquodo::ts::Comments;

let comments = Comments::new();
let description = "The pet's name.";

let ast = ts_quote!(
    comments,
    "/** #{desc} */ name: string" as TsTypeElement,
    desc: &str = description,
);
```

The optional `comments` parameter collects comments for code generation. Rendering them requires [`swc_ecma_codegen`](https://rustdoc.swc.rs/swc_ecma_codegen/), added as a separate dependency:

```rust
use swc_ecma_codegen::to_code_with_comments; // From the `swc_ecma_codegen` crate.

let comments = Comments::new();
let noun = "pet's name";
let adjective = "required";

let ast = ts_quote!(
    comments,
    "/** The #{noun} is #{adjective}. */ name: string" as TsTypeElement,
    noun: &str = noun,
    adjective: &str = adjective,
);

let code = to_code_with_comments(Some(&*comments), &ast);
// => `/** The pet's name is required. */ name: string;`
```

For more complex uses, `JsDoc` variables attach pre-built JSDoc comments to nodes. Each comment attaches to the syntax tree node that follows it:

```rust
use quasiquodo::ts::{Comments, JsDoc};
use swc_ecma_codegen::to_code_with_comments;

let comments = Comments::new();
let doc = JsDoc::new("The pet's name.");

let ast = ts_quote!(
    comments,
    "export interface Pet { #{doc} name: string; }" as ModuleItem,
    doc: JsDoc = doc,
);

let code = to_code_with_comments(Some(&*comments), &ast);
```

This produces:

```typescript
export interface Pet {
    /** The pet's name. */ name: string;
}
```

`JsDoc` variables can also be embedded in comment text:

```rust
let doc = JsDoc::new("a pet");
let ast = ts_quote!(
    comments,
    "/** This is #{doc}. */ name: string" as TsTypeElement,
    doc: JsDoc = doc,
);
// => `/** This is a pet. */ name: string;`
```

`Option<JsDoc>`, `Option<&str>`, and `Option<String>` conditionally attach comments. `None` emits no comment:

```rust
let doc = if include_docs {
    Some(JsDoc::new("The pet's name."))
} else {
    None
};

let ast = ts_quote!(
    comments,
    "#{doc} name: string" as TsTypeElement,
    doc: Option<JsDoc> = doc,
);
// `/** The pet's name. */ name: string;` or `name: string;`,
// depending on `doc`.
```

`JsDoc` variables are preserved across nested `ts_quote!` calls, so you can document a member first, then splice it into a larger structure:

```rust
let comments = Comments::new();
let doc = JsDoc::new("The pet's name.");

// Attach the comment to a member...
let member = ts_quote!(
    comments,
    "#{doc} name: string" as ClassMember,
    doc: JsDoc = doc,
);

// ...then splice the member into a class.
let class = ts_quote!(
    "class Pet { #{member} }" as Stmt,
    member: ClassMember = member,
);

let code = to_code_with_comments(Some(&*comments), &class);
```

This produces:

```typescript
class Pet {
    /** The pet's name. */ name: string;
}
```

## Reference

### Output kinds

The output kind tells `ts_quote!` which [`swc_ecma_ast`](https://rustdoc.swc.rs/swc_ecma_ast/) type to return.

| Output kind | AST type | Example source |
|-------------|----------|----------------|
| `TsType` | `TsType` | `"string \| null"` |
| `Expr` | `Expr` | `"foo()"` |
| `Stmt` | `Stmt` | `"return x;"` |
| `Decl` | `Decl` | `"type T = string;"` |
| `ModuleItem` | `ModuleItem` | `"export interface Pet {}"` |
| `Ident` | `Ident` | `"MyType"` |
| `TsTypeElement` | `TsTypeElement` | `"name: string"` |
| `ClassMember` | `ClassMember` | `"greet() {}"` |
| `Param` | `Param` | `"x: number"` |
| `ParamOrTsParamProp` | `ParamOrTsParamProp` | `"public name: string"` |
| `ImportSpecifier` | `ImportSpecifier` | `"Foo as Bar"` |
| `ExportSpecifier` | `ExportSpecifier` | `"Foo as Bar"` |

### Variable types

Variables can be scalar, boxed, or container types.

**Scalar types** substitute a single node or literal value:

| Variable type | Rust value type | Description |
|---------------|-----------------|-------------|
| `TsType` | `TsType` | A TypeScript type |
| `Expr` | `Expr` | An expression |
| `Ident` | `Ident` | An identifier |
| `Stmt` | `Stmt` | A statement |
| `TsTypeElement` | `TsTypeElement` | An interface member |
| `ClassMember` | `ClassMember` | A class member |
| `Param` | `Param` | A function parameter |
| `ParamOrTsParamProp` | `ParamOrTsParamProp` | A constructor parameter |
| `ImportSpecifier` | `ImportSpecifier` | An import specifier |
| `ExportSpecifier` | `ExportSpecifier` | An export specifier |
| `Decl` | `Decl` | A declaration |
| `JsDoc` | `JsDoc` | A pre-built JSDoc comment |
| `&str` | `&str` | A string slice, in literal position |
| `String` | `String` | An owned string, in literal position |
| `f64` | `f64` | A floating-point number, in literal position |
| `usize` | `usize` | An integer, in literal position |
| `bool` | `bool` | A Boolean, in literal position |

**Container types** wrap any scalar type:

| Container | Behavior |
|-----------|----------|
| `Box<T>` | A boxed scalar |
| `Vec<T>` | Splices zero or more items into a list position |
| `Option<T>` | Conditionally splices one item or nothing |

## How It Works

`ts_quote!` is a procedural macro that expands into a pure Rust block expression—no runtime parsing, just construction code. All TypeScript parsing happens at compile time.

When the macro runs, it replaces `#{var}` placeholders with syntactically appropriate stand-ins, so that the preprocessed source becomes valid TypeScript. It then parses that source with `swc_ecma_parser`, and extracts the requested output type from the AST.

The interesting part comes next: Quasiquodo _unparses_ the AST, turning each node into a Rust expression that constructs the equivalent node in your program. Along the way, it replaces the stand-ins with the bound variables. The result is Rust code that builds the AST directly.

## Contributing

We love contributions!

If you find a case where Quasiquodo fails, generates incorrect output, or lacks an output kind you need, please [open an issue](https://github.com/linabutler/quasiquodo/issues/new) with a minimal reproducing `ts_quote!` invocation.

For questions or larger contributions, please [start a discussion](https://github.com/linabutler/quasiquodo/discussions).

Quasiquodo follows [the Ghostty project's AI Usage policy](https://github.com/ghostty-org/ghostty/blob/1fa4e787eb1f50729153d09b7f455ebb9fc4ccc9/AI_POLICY.md).

## Acknowledgments

Quasiquodo builds on the excellent [**SWC**](https://swc.rs) project, whose [parser](https://rustdoc.swc.rs/swc_ecma_parser/) and [AST](https://rustdoc.swc.rs/swc_ecma_ast/) make it all possible, and whose [quasi-quotation macro for JavaScript](https://rustdoc.swc.rs/swc_ecma_quote/) inspired Quasiquodo's design.
