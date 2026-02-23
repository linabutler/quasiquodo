use proc_macro::TokenStream;

#[doc(hidden)]
#[proc_macro]
pub fn py_quote_with_root(input: TokenStream) -> TokenStream {
    quasiquodo_py_core::expand(input.into()).into()
}
