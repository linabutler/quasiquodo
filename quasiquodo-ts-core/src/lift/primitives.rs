use num_bigint::Sign;
use proc_macro2::Span;
use swc_atoms::{Atom, Wtf8Atom};
use swc_common::SyntaxContext;
use swc_ecma_ast::*;
use syn::parse_quote;

use crate::{
    context::{Context, UnboundVar, VarData},
    input::VarType,
    lexer::stand_ins::{StandInScanner, StandInToken},
    lift::unsplice,
};

use super::{CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, lift_variants};

impl Lift for swc_common::Span {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let comment = context.take_closest_doc_to(self.lo);
        let expr = match comment.and_then(|comment| context.comments().map(|expr| (expr, comment)))
        {
            // If this span has an attached JSDoc comment, and we can save it in
            // `comments`, emit a `comments.span_with_comment(...)` expression.
            Some((expr, comment)) => {
                let text = &*comment.text;
                let segments: Vec<_> = StandInScanner::new(text).collect();

                // Build a format string, with `{}` placeholders for each
                // stand-in, and collect the corresponding bound
                // variables for them.
                let mut format_str = String::new();
                let mut vars = vec![];
                for &segment in &segments {
                    match segment {
                        StandInToken::Text(t) => format_str.push_str(t),
                        StandInToken::StandIn(p) => match context.stand_in(p) {
                            Some(var) => {
                                format_str.push_str("{}");
                                vars.push(var);
                            }
                            None => {
                                return Err(syn::Error::new(
                                    Span::call_site(),
                                    UnboundVar(p.to_owned()),
                                ));
                            }
                        },
                    }
                }

                match &*vars {
                    // Static comment without stand-ins; use the text as-is.
                    [] => {
                        parse_quote!(
                            ::quasiquodo::ts::Comments::span_with_comment(
                                &#expr,
                                #text,
                            )
                        )
                    }
                    // A single `Option<&str | String | JsDoc>` variable uses
                    // the text if `Some`, or removes the entire comment
                    // if `None`.
                    [
                        VarData {
                            ident,
                            ty: VarType::Option(ty),
                        },
                    ] if {
                        segments
                            .iter()
                            .filter_map(|segment| match segment {
                                StandInToken::Text(t) => Some(t.trim()),
                                _ => None,
                            })
                            .all(|t| t.is_empty() || t == "*")
                    } =>
                    {
                        let fallback = context.span();
                        let format_arg: syn::Expr = if ty.is_str() {
                            parse_quote!(doc)
                        } else {
                            // The lexer restricts comment variable types
                            // to strings and `JsDoc`s, so this must be a
                            // `JsDoc`.
                            parse_quote!(doc.raw_text())
                        };
                        parse_quote!(
                            match #ident {
                                Some(ref doc) => ::quasiquodo::ts::Comments::span_with_comment(
                                    &#expr,
                                    format!(#format_str, #format_arg),
                                ),
                                None => #fallback,
                            }
                        )
                    }
                    // Other combinations of embedded stand-ins produce a
                    // `format!` expression, with bound variables as arguments.
                    other => {
                        let format_args = other.iter().map(|VarData { ident, ty }| -> syn::Expr {
                            match ty {
                                VarType::JsDoc => parse_quote!(#ident.raw_text()),
                                VarType::Option(inner) if inner.is_str() => {
                                    parse_quote!(#ident.unwrap_or_default())
                                }
                                VarType::Option(inner) if matches!(**inner, VarType::JsDoc) => {
                                    parse_quote!(
                                        #ident
                                            .as_ref()
                                            .map(|d| d.raw_text())
                                            .unwrap_or_default()
                                    )
                                }
                                _ => parse_quote!(#ident),
                            }
                        });
                        parse_quote!(
                            ::quasiquodo::ts::Comments::span_with_comment(
                                &#expr,
                                format!(#format_str, #(#format_args),*),
                            )
                        )
                    }
                }
            }
            // No `comments`, so nowhere to save the comment; discard it.
            None => context.span(),
        };
        Ok(CodeFragment::Single(expr))
    }
}

impl Lift for SyntaxContext {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        Ok(CodeFragment::Single(parse_quote!(
            ::quasiquodo::ts::swc::common::SyntaxContext::empty()
        )))
    }
}

impl Lift for bool {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        Ok(CodeFragment::Single(parse_quote!(#self)))
    }
}

impl Lift for f64 {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        Ok(CodeFragment::Single(parse_quote!(#self)))
    }
}

impl Lift for Atom {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        let val = &**self;
        Ok(CodeFragment::Single(parse_quote!(
            ::quasiquodo::ts::swc::atoms::atom!(#val)
        )))
    }
}

impl Lift for Wtf8Atom {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        let bytes = syn::LitByteStr::new(self.as_bytes(), Span::call_site());
        Ok(CodeFragment::Single(parse_quote!(unsafe {
            // Safety: `bytes` came from a `Wtf8Atom` created at compile time.
            ::quasiquodo::ts::swc::atoms::Wtf8Atom::from_bytes_unchecked(#bytes)
        })))
    }
}

impl<T: Lift> Lift for Box<T> {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let inner = unsplice!((**self).lift(context)?);
        Ok(CodeFragment::Single(parse_quote!(Box::new(#inner))))
    }
}

impl<T: Lift> Lift for Option<T> {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        match self {
            Some(inner) => match inner.lift(context)? {
                CodeFragment::Splice(expr) => Ok(CodeFragment::Splice(
                    parse_quote!((#expr).map(|v| Some(Into::into(v)))),
                )),
                CodeFragment::Single(expr) => Ok(CodeFragment::Single(parse_quote!(Some(#expr)))),
            },
            None => Ok(CodeFragment::Single(parse_quote!(None))),
        }
    }
}

impl<T: Lift> Lift for Vec<T> {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let stmts = self
            .iter()
            .map(|item| {
                let expr: syn::Expr = match item.lift(context)? {
                    CodeFragment::Single(expr) => parse_quote!(items.push(#expr)),
                    CodeFragment::Splice(expr) => {
                        parse_quote!(items.extend((#expr).map(Into::into)))
                    }
                };
                Ok(expr)
            })
            .collect::<syn::Result<Vec<_>>>()?;
        let capacity = stmts.len();
        Ok(CodeFragment::Single(parse_quote! {{
            let mut items = Vec::with_capacity(#capacity);
            #(#stmts;)*
            items
        }}))
    }
}

impl Lift for Ident {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        match context.stand_in(&self.sym) {
            Some(var) => {
                let var_ident = &var.ident;
                Ok(match &var.ty {
                    VarType::Ident => CodeFragment::Single(parse_quote!(#var_ident.clone())),
                    VarType::Vec(_) | VarType::Option(_) => {
                        CodeFragment::Splice(parse_quote!(#var_ident.iter().cloned()))
                    }
                    _ => {
                        // This identifier is an antiquotation: a stand-in for
                        // a node that needs to be replaced further up the tree
                        // (e.g., if the variable is a `ClassMember`, it needs to
                        // replace the `ClassMember` that contains this `Ident`,
                        // not the `Ident` itself). Propagate the identifier upward
                        // until it reaches its target position.
                        CodeFragment::Splice(parse_quote!(std::iter::once(#var_ident.clone())))
                    }
                })
            }
            None => {
                let sym = unsplice!(self.sym.lift(context)?);
                let span = unsplice!(self.span.lift(context)?);
                let expr = if self.optional {
                    parse_quote!(::quasiquodo::ts::swc::ecma_ast::Ident {
                        sym: #sym,
                        span: #span,
                        ctxt: ::quasiquodo::ts::swc::common::SyntaxContext::empty(),
                        optional: true,
                    })
                } else {
                    parse_quote!(::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                        #sym,
                        #span,
                    ))
                };
                Ok(CodeFragment::Single(expr))
            }
        }
    }
}

impl Lift for IdentName {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        match context.stand_in(&self.sym) {
            Some(var) => {
                let var_ident = &var.ident;
                Ok(match &var.ty {
                    VarType::Ident => {
                        // Convert `Ident` variables to `IdentName`s.
                        let span = context.span();
                        CodeFragment::Single(
                            parse_quote!(::quasiquodo::ts::swc::ecma_ast::IdentName {
                                span: #span,
                                sym: ::quasiquodo::ts::swc::atoms::Atom::clone(&#var_ident.sym),
                            }),
                        )
                    }
                    // Same as for `impl Ident` above.
                    VarType::Vec(_) | VarType::Option(_) => {
                        CodeFragment::Splice(parse_quote!(#var_ident.iter().cloned()))
                    }
                    _ => CodeFragment::Splice(parse_quote!(std::iter::once(#var_ident.clone()))),
                })
            }
            None => {
                let span = unsplice!(self.span.lift(context)?);
                let sym = unsplice!(self.sym.lift(context)?);
                Ok(CodeFragment::Single(
                    parse_quote!(::quasiquodo::ts::swc::ecma_ast::IdentName {
                        span: #span,
                        sym: #sym,
                    }),
                ))
            }
        }
    }
}

/// Custom implementation to splice string variables.
impl Lift for Str {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let expr = if let Some(value) = self.value.as_str()
            && let Some(var) = context.stand_in(value)
            && var.ty.is_str()
        {
            let var_ident = &var.ident;
            let span = context.span();
            parse_quote!(::quasiquodo::ts::swc::ecma_ast::Str {
                span: #span,
                value: ::quasiquodo::ts::swc::atoms::Wtf8Atom::new(#var_ident.clone()),
                raw: None,
            })
        } else {
            let span = unsplice!(self.span.lift(context)?);
            let value = unsplice!(self.value.lift(context)?);
            let raw = unsplice!(self.raw.lift(context)?);
            parse_quote!(::quasiquodo::ts::swc::ecma_ast::Str {
                span: #span,
                value: #value,
                raw: #raw,
            })
        };
        Ok(CodeFragment::Single(expr))
    }
}

impl_lift_for_struct!(Bool, [span, value]);

impl_lift_for_struct!(Number, [span, value, raw]);

impl_lift_for_struct!(Null, [span]);

impl_lift_for_struct!(Regex, [span, exp, flags]);

impl Lift for num_bigint::BigInt {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        let (sign_ident, digits) = {
            let (sign, digits) = self.to_u32_digits();
            let sign_ident = syn::Ident::new(
                match sign {
                    Sign::Minus => "Minus",
                    Sign::NoSign => "NoSign",
                    Sign::Plus => "Plus",
                },
                Span::call_site(),
            );
            (sign_ident, digits)
        };
        Ok(CodeFragment::Single(
            parse_quote!(::quasiquodo::ts::num_bigint::BigInt::from_slice(
                ::quasiquodo::ts::num_bigint::Sign::#sign_ident,
                &[#(#digits),*],
            )),
        ))
    }
}

impl_lift_for_struct!(BigInt, [span, value, raw]);

impl_lift_for_newtype_enum!(Lit, [Str, Bool, Null, Num, BigInt, Regex, JSXText]);
