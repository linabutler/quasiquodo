use num_bigint::Sign;
use proc_macro2::Span;
use swc_atoms::{Atom, Wtf8Atom};
use swc_common::SyntaxContext;
use swc_ecma_ast::*;
use syn::parse_quote;

use crate::{
    context::Context,
    input::VarType,
    lexer::docs::{self, CommentSegment},
    lift::unsplice,
};

use super::{CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, lift_variants};

impl Lift for swc_common::Span {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let expr = match context
            .take_closest_doc_to(self.lo)
            .and_then(|comment| context.comments().map(|expr| (expr, comment)))
        {
            // If this span has an attached JSDoc comment, and we can save it in
            // `comments`, emit a `comments.span_with_comment(...)` expression.
            Some((expr, comment)) => {
                let text = &*comment.text;
                let segments = docs::segments(text);

                match segments.iter().find_map(|s| match s {
                    CommentSegment::Placeholder(p) => {
                        context.placeholder(p).filter(|var| match &var.ty {
                            VarType::JsDoc => true,
                            VarType::Option(inner) => matches!(**inner, VarType::JsDoc),
                            _ => false,
                        })
                    }
                    _ => None,
                }) {
                    // A `JsDoc` or `Option<JsDoc>` variable.
                    Some(var) => {
                        let var_ident = var.to_tokens();
                        match &var.ty {
                            VarType::Option(inner) if matches!(**inner, VarType::JsDoc) => {
                                // For `Option<JsDoc>`, only attach a comment
                                // for `Some`; ignore `None`.
                                let fallback = context.span();
                                parse_quote!(
                                    match #var_ident {
                                        Some(ref doc) => ::quasiquodo::ts::Comments::span_with_comment(
                                            &#expr,
                                            doc.text(),
                                        ),
                                        None => #fallback,
                                    }
                                )
                            }
                            _ => parse_quote!(
                                ::quasiquodo::ts::Comments::span_with_comment(
                                    &#expr,
                                    #var_ident.text(),
                                )
                            ),
                        }
                    }
                    None => {
                        if segments
                            .iter()
                            .any(|s| matches!(s, CommentSegment::Placeholder(_)))
                        {
                            // For segments with placeholders, build a
                            // dynamic `format!(...)` string, interpolating
                            // the values of the corresponding variables.
                            let mut format_str = String::new();
                            let mut format_args: Vec<syn::Expr> = Vec::new();
                            for segment in segments {
                                match segment {
                                    CommentSegment::Text(t) => format_str.push_str(t),
                                    CommentSegment::Placeholder(p) => {
                                        match context.placeholder(p) {
                                            // Emit format string placeholders and
                                            // arguments for bound variables.
                                            Some(var) => {
                                                let var_ident = var.to_tokens();
                                                let arg = match &var.ty {
                                                    VarType::Option(inner)
                                                        if matches!(**inner, VarType::LitStr) =>
                                                    {
                                                        // For `Option<LitStr>`, substitute
                                                        // an empty string for `None`.
                                                        parse_quote!(#var_ident.unwrap_or_default())
                                                    }
                                                    _ => parse_quote!(#var_ident),
                                                };
                                                format_str.push_str("{}");
                                                format_args.push(arg);
                                            }
                                            // Unbound variable; insert the placeholder as-is.
                                            None => format_str.push_str(p),
                                        }
                                    }
                                }
                            }
                            parse_quote!(
                                ::quasiquodo::ts::Comments::span_with_comment(
                                    &#expr,
                                    format!(#format_str, #(#format_args),*),
                                )
                            )
                        } else {
                            // For static comments, or comments with placeholders
                            // that aren't bound to variables, just use the text.
                            parse_quote!(
                                ::quasiquodo::ts::Comments::span_with_comment(
                                    &#expr,
                                    #text,
                                )
                            )
                        }
                    }
                }
            }
            // No `comments` argument, so nowhere to save the comment;
            // discard it.
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
        match context.placeholder(&self.sym) {
            Some(var) => {
                let var_ident = var.to_tokens();
                Ok(match &var.ty {
                    VarType::Ident => CodeFragment::Single(parse_quote!(#var_ident)),
                    VarType::Vec(_) | VarType::Option(_) => {
                        CodeFragment::Splice(parse_quote!(#var_ident.into_iter()))
                    }
                    _ => {
                        // This identifier is an antiquotation: a stand-in for
                        // a node that needs to be replaced further up the tree
                        // (e.g., if the variable is a `ClassMember`, it needs to
                        // replace the `ClassMember` that contains this `Ident`,
                        // not the `Ident` itself). Propagate the identifier upward
                        // until it reaches its target position.
                        CodeFragment::Splice(parse_quote!(std::iter::once(#var_ident)))
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
        match context.placeholder(&self.sym) {
            Some(var) => {
                let var_ident = var.to_tokens();
                Ok(match &var.ty {
                    VarType::Ident => {
                        // Convert `Ident` variables to `IdentName`s.
                        let var_ident = var.to_tokens();
                        let span = context.span();
                        CodeFragment::Single(
                            parse_quote!(::quasiquodo::ts::swc::ecma_ast::IdentName {
                                span: #span,
                                sym: #var_ident.sym,
                            }),
                        )
                    }
                    // Same as for `impl Ident` above.
                    VarType::Vec(_) | VarType::Option(_) => {
                        CodeFragment::Splice(parse_quote!(#var_ident.into_iter()))
                    }
                    _ => CodeFragment::Splice(parse_quote!(std::iter::once(#var_ident))),
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

/// Custom implementation to detect preprocessed `LitStr` placeholders
/// (e.g., `"__tsq_0__"`) and substitute the variable's value.
impl Lift for Str {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let expr = if let Some(value) = self.value.as_str()
            && let Some(var) = context.placeholder(value)
            && matches!(var.ty, VarType::LitStr)
        {
            let var_ident = var.to_tokens();
            let span = context.span();
            parse_quote!(::quasiquodo::ts::swc::ecma_ast::Str {
                span: #span,
                value: (#var_ident).into(),
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
