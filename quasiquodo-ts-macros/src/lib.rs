use proc_macro::TokenStream;

#[doc(hidden)]
#[proc_macro]
pub fn ts_quote_with_root(input: TokenStream) -> TokenStream {
    quasiquodo_ts_core::expand(input.into()).into()
}
