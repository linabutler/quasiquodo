use proc_macro::TokenStream;

/// Parses a TypeScript source string at compile time and expands to
/// Rust code that constructs the equivalent `swc_ecma_ast` nodes.
///
/// All span fields in the generated AST are set to `DUMMY_SP` by
/// default. Pass `span = expr` before the source string to use a
/// custom span instead.
///
/// ```ignore
/// use quasiquodo::ts_quote;
///
/// let item = ts_quote!(r#"export type Status = "active" | "inactive";"# as ModuleItem);
/// ```
#[proc_macro]
pub fn ts_quote(input: TokenStream) -> TokenStream {
    quasiquodo_ts_core::expand(input.into()).into()
}
