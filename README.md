# Quasiquodo

**Compile-time TypeScript quasi-quoting for Rust.**

Quasiquodo is a Rust macro that turns inline TypeScript into correct-by-construction syntax trees, giving you TypeScript ergonomics with compile-time safety.

Instead of writing:

```rust
let ty = TsType::TsUnionOrIntersectionType(
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

Quasiquodo lets you write:

```rust
let ty = ts_quote!("string | null" as TsType);
```

## Getting Started

Add Quasiquodo to your `Cargo.toml`:

```toml
[dependencies]
quasiquodo = "0.1"
```

Then import the `ts_quote!` macro, and any [SWC](https://swc.rs) types that you need:

```rust
use quasiquodo::ts_quote;
use quasiquodo::ts::swc::ecma_ast::*;
```

Quasiquodo's minimum supported Rust version (MSRV) is **Rust 1.89.0**. The MSRV may increase in minor releases (e.g., Quasiquodo 1.1.x may require a newer MSRV than 1.0.x).

## Usage

### Basic quoting

The `ts_quote!` macro takes a TypeScript source string and an output kind, and returns the corresponding [`swc_ecma_ast`](https://rustdoc.swc.rs/swc_ecma_ast/) type:

```rust
let ty: TsType = ts_quote!("string | null" as TsType);
```

The output kind, like `TsType`, `Expr`, or `ModuleItem`, tells `ts_quote!` how to parse the source, and which type of syntax tree node to return. You can quote any TypeScript construct that has a corresponding output kind:

```rust
let ty: TsType = ts_quote!("Record<string, number>" as TsType);

let expr: Expr = ts_quote!("foo()" as Expr);

let decl = ts_quote!("export interface Pet { name: string; age?: number; }" as ModuleItem);
```

This syntax is inspired by the [`swc_ecma_quote`](https://rustdoc.swc.rs/swc_ecma_quote/macro.quote.html) macro.

### Variable substitution

You can use `$binding` placeholders to splice variables into the TypeScript syntax trees that `ts_quote!` builds. Each variable is declared with a name, type, and value, and replaces the placeholder at compile time:

```rust
let name: Ident = ts_quote!("Pet" as Ident);
let field_type: TsType = ts_quote!("string[]" as TsType);

let ast = ts_quote!(
    "$name: $field_type" as TsTypeElement,
    name: Ident = name,
    field_type: TsType = field_type,
);
```

`LitStr`, `LitNum`, and `LitBool` variables replace their placeholders with their respective values. For example, to use a `LitStr` variable in a property name or member access position:

```rust
let name = "color";
let ast = ts_quote!("$name: string" as TsTypeElement, name: LitStr = name);
// => `color: string;`

let field = "name";
let ast: Expr = ts_quote!("foo[$f]" as Expr, f: LitStr = field);
// => `foo.name`
```

`LitStr` variables in these positions are simplified when their values are valid identifiers, and quoted when they're not:

```rust
let name = "background-color";
let ast = ts_quote!("$name: string" as TsTypeElement, name: LitStr = name);
// => `"background-color": string;`

let field = "some-field";
let ast: Expr = ts_quote!("foo[$f]" as Expr, f: LitStr = field);
// => `foo["some-field"]`
```

Placeholders can occur in any position, even where TypeScript wouldn't normally allow identifiers:

```rust
let module = "./types";
let ast = ts_quote!(
    "import type { Pet } from $module;" as ModuleItem,
    module: LitStr = module,
);
// => `import type { Pet } from "./types";`
```

To include a literal `$` in the output, use `$$`:

```rust
let ast: Expr = ts_quote!("$$foo" as Expr);
// => `$foo`
```

`$$` escapes work in JSDoc comments, too:

```rust
let ast: TsTypeElement = ts_quote!(
    comments,
    "/** See $$ref for details. */ name: string" as TsTypeElement,
);
// => `/** See $ref for details. */ name: string;`
```

### Splicing

`Vec<T>` variables splice naturally into list positions:

- Union and intersection type arms.
- Interface `extends` clauses.
- Interface and class bodies.
- Function and constructor parameter lists.
- Call expression arguments.
- Array literal elements.
- Import and export specifier lists.
- Block statement bodies.

```rust
let name: Ident = ts_quote!("Pet" as Ident);
let members: Vec<TsTypeElement> = vec![
    ts_quote!("name: string" as TsTypeElement),
    ts_quote!("age?: number" as TsTypeElement),
];

let ast = ts_quote!(
    "export interface $N { $M; }" as ModuleItem,
    N: Ident = name,
    M: Vec<TsTypeElement> = members,
);
```

This produces:

```typescript
export interface Pet {
    name: string;
    age?: number;
}
```

Some positions, like union and intersection types, require `Box<T>` wrapping:

```rust
let extra = vec![
    Box::new(ts_quote!("number" as TsType)),
    Box::new(ts_quote!("boolean" as TsType)),
];

let ty: TsType = ts_quote!(
    "string | $Extra" as TsType,
    Extra: Vec<Box<TsType>> = extra,
);
// => `string | number | boolean`
```

`Option<T>` conditionally includes a single element:

```rust
let extra: Option<TsTypeElement> = if include_age {
    Some(ts_quote!("age?: number" as TsTypeElement))
} else {
    None
};

let ast = ts_quote!(
    "export interface Pet { name: string; $extra; }" as ModuleItem,
    extra: Option<TsTypeElement> = extra,
);
```

### Custom spans

The optional `span` parameter applies a custom [`Span`](https://rustdoc.swc.rs/swc_core/common/struct.Span.html) to all nodes in the returned syntax tree:

```rust
use quasiquodo::ts::swc::common::{BytePos, Span};

let ast = ts_quote!(
    span = Span::new(BytePos(10), BytePos(25)),
    "name: string" as TsTypeElement,
);
```

This is useful for error reporting, so that diagnostics point to the right location in your source.

### JSDoc comments

`ts_quote!` understands JSDoc-style `/** ... */` comments, and supports splicing `LitStr` variables into them:

```rust
let description = "The pet's name.";
let ast = ts_quote!(
    "/** $desc */ name: string" as TsTypeElement,
    desc: LitStr = description,
);
```

The optional `comments` parameter collects comments for code generation. Rendering the comments requires [`swc_ecma_codegen`](https://rustdoc.swc.rs/swc_ecma_codegen/), which you'll need to add as a separate dependency:

```rust
use quasiquodo::ts::Comments;
use swc_ecma_codegen::to_code_with_comments; // From the `swc_ecma_codegen` crate.

let comments = Comments::new();
let noun = "pet's name";
let adjective = "required";
let ast = ts_quote!(
    comments,
    "/** The $noun is $adjective. */ name: string" as TsTypeElement,
    noun: LitStr = noun,
    adjective: LitStr = adjective,
);

let code = to_code_with_comments(Some(&*comments), &ast);
// => `/** The pet's name is required. */ name: string;`
```

## Reference

### Output kinds

The output kind indicates which [`swc_ecma_ast`](https://rustdoc.swc.rs/swc_ecma_ast/) type to parse from the source.

| Output kind | AST type | Example source |
|-------------|----------|----------------|
| `TsType` | `TsType` | `"string \| null"` |
| `Expr` | `Expr` | `"foo()"` |
| `Stmt` | `Stmt` | `"return x;"` |
| `Decl` | `Decl` | `"type T = string;"` |
| `ModuleItem` | `ModuleItem` | `"export interface Pet { }"` |
| `Ident` | `Ident` | `"MyType"` |
| `TsTypeElement` | `TsTypeElement` | `"name: string"` |
| `ClassMember` | `ClassMember` | `"greet() { }"` |
| `Param` | `Param` | `"x: number"` |
| `ParamOrTsParamProp` | `ParamOrTsParamProp` | `"public name: string"` |
| `ImportSpecifier` | `ImportSpecifier` | `"Foo as Bar"` |
| `ExportSpecifier` | `ExportSpecifier` | `"Foo as Bar"` |

### Variable types

Variables declared with `$binding` can have scalar, boxed, or container types.

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
| `LitStr` | `&str` | A string literal value |
| `LitNum` | `f64` | A numeric literal value |
| `LitBool` | `bool` | A boolean literal value |

**Container types** wrap any scalar type:

| Container | Behavior |
|-----------|----------|
| `Box<T>` | A boxed scalar |
| `Vec<T>` | Splices zero or more items into a list position |
| `Option<T>` | Conditionally splices one item or nothing |

## How It Works

`ts_quote!` expands to a block expression that evaluates to the constructed AST node. Parsing happens entirely at compile time; the generated Rust code is pure construction.

To do this, Quasiquodo:

1. Parses the Rust macro input, extracting the source, output kind, and optional variable declarations.
2. **Replaces `$binding` placeholders** with syntactically appropriate stand-ins, so that the result is valid TypeScript.
3. **Parses the preprocessed source** with `swc_ecma_parser`, and extracts the output `swc_ecma_ast` type from the syntax tree.
4. **Generates Rust code** from the output, "unparsing" each syntax tree node into a `syn::Expr` that constructs the equivalent `swc_ecma_ast` type, and replacing placeholder nodes with the actual variables passed to the macro.

## Contributing

We love contributions!

If you find a case where Quasiquodo fails, generates incorrect output, or doesn't support an output `swc_ecma_ast` type that you need, please [open an issue](https://github.com/linabutler/quasiquodo/issues/new) with a minimal reproducing `ts_quote!` invocation.

For questions, or for planning larger contributions, please [start a discussion](https://github.com/linabutler/quasiquodo/discussions).

Quasiquodo follows [the Ghostty project's AI Usage policy](https://github.com/ghostty-org/ghostty/blob/1fa4e787eb1f50729153d09b7f455ebb9fc4ccc9/AI_POLICY.md).

## Acknowledgments

Quasiquodo builds on the excellent work of the [**SWC**](https://swc.rs) project, whose [parser](https://rustdoc.swc.rs/swc_ecma_parser/) and [AST](https://rustdoc.swc.rs/swc_ecma_ast/) make the whole thing possible, and whose [quasi-quotation macro for JavaScript](https://rustdoc.swc.rs/swc_ecma_quote/) inspired Quasiquodo's design.
