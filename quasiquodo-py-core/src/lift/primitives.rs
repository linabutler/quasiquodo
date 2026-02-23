use proc_macro2::Span;
use ruff_python_ast::*;
use ruff_text_size::TextRange;
use syn::parse_quote;

use crate::{
    context::{Context, UnboundVar},
    input::VarType,
    lexer::stand_ins::{StandInScanner, StandInToken},
};

use super::{
    CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, impl_lift_for_unit_enum,
    lift_variants, unsplice,
};

// MARK: Support types

impl Lift for TextRange {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        Ok(CodeFragment::Single(parse_quote!(
            #root::ruff::text_size::TextRange::default()
        )))
    }
}

impl Lift for AtomicNodeIndex {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        Ok(CodeFragment::Single(parse_quote!(
            #root::ruff::python_ast::AtomicNodeIndex::NONE
        )))
    }
}

// MARK: Primitives

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

impl Lift for u32 {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        Ok(CodeFragment::Single(parse_quote!(#self)))
    }
}

// MARK: Strings

impl Lift for name::Name {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        let val = self.as_str();
        Ok(CodeFragment::Single(parse_quote!(
            #root::ruff::python_ast::name::Name::new(#val)
        )))
    }
}

impl Lift for Box<str> {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        let val = &**self;
        Ok(CodeFragment::Single(parse_quote!(Box::from(#val))))
    }
}

// MARK: Generic containers

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

/// [`ExprCompare`] uses boxed slices for `ops` and `comparators`.
impl<T: Lift> Lift for Box<[T]> {
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
            items.into_boxed_slice()
        }}))
    }
}

// MARK: `ExprContext`

impl_lift_for_unit_enum!(ExprContext, [Load, Store, Del, Invalid]);

// MARK: Identifiers

impl Lift for Identifier {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        match context.stand_in(self.id.as_str()) {
            Some(var) => {
                let var_ident = &var.ident;
                Ok(match &var.ty {
                    VarType::Identifier => CodeFragment::Single(parse_quote!(#var_ident.clone())),
                    VarType::Vec(_) | VarType::Option(_) => {
                        CodeFragment::Splice(parse_quote!(#var_ident.iter().cloned()))
                    }
                    _ => CodeFragment::Splice(parse_quote!(std::iter::once(#var_ident.clone()))),
                })
            }
            None => {
                let id = unsplice!(self.id.lift(context)?);
                let range = unsplice!(self.range.lift(context)?);
                let node_index = unsplice!(self.node_index.lift(context)?);
                let root = context.root();
                Ok(CodeFragment::Single(
                    parse_quote!(#root::ruff::python_ast::Identifier {
                        id: #id,
                        range: #range,
                        node_index: #node_index,
                    }),
                ))
            }
        }
    }
}

impl Lift for ExprName {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        match context.stand_in(self.id.as_str()) {
            Some(var) => {
                let var_ident = &var.ident;
                Ok(match &var.ty {
                    VarType::Identifier => {
                        // Convert `Identifier` variables to `ExprName`s.
                        CodeFragment::Single(parse_quote!(#root::ruff::python_ast::ExprName {
                            node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                            range: #root::ruff::text_size::TextRange::default(),
                            id: #root::ruff::python_ast::name::Name::clone(&#var_ident.id),
                            ctx: #root::ruff::python_ast::ExprContext::Load,
                        }))
                    }
                    VarType::Vec(_) | VarType::Option(_) => {
                        CodeFragment::Splice(parse_quote!(#var_ident.iter().cloned()))
                    }
                    _ => CodeFragment::Splice(parse_quote!(std::iter::once(#var_ident.clone()))),
                })
            }
            None => {
                let node_index = unsplice!(self.node_index.lift(context)?);
                let range = unsplice!(self.range.lift(context)?);
                let id = unsplice!(self.id.lift(context)?);
                let ctx = unsplice!(self.ctx.lift(context)?);
                Ok(CodeFragment::Single(
                    parse_quote!(#root::ruff::python_ast::ExprName {
                        node_index: #node_index,
                        range: #range,
                        id: #id,
                        ctx: #ctx,
                    }),
                ))
            }
        }
    }
}

// MARK: String literals

impl Lift for StringLiteral {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        let value = &*self.value;

        // Check for `__pyq_N__` stand-ins embedded in the string value.
        let stand_ins: Vec<_> = StandInScanner::new(value).collect();
        if stand_ins
            .iter()
            .any(|s| matches!(s, StandInToken::StandIn(_)))
        {
            let restored = format_expr_with_stand_ins(&stand_ins, context)?;
            return Ok(CodeFragment::Single(
                parse_quote!(#root::ruff::python_ast::StringLiteral {
                    range: #root::ruff::text_size::TextRange::default(),
                    node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                    value: Box::from(#restored),
                    flags: #root::ruff::python_ast::StringLiteralFlags::empty(),
                }),
            ));
        }

        let range = unsplice!(self.range.lift(context)?);
        let node_index = unsplice!(self.node_index.lift(context)?);
        let flags = unsplice!(self.flags.lift(context)?);
        Ok(CodeFragment::Single(
            parse_quote!(#root::ruff::python_ast::StringLiteral {
                range: #range,
                node_index: #node_index,
                value: Box::from(#value),
                flags: #flags,
            }),
        ))
    }
}

/// Builds a `format!` expression from a sequence of [`StandInToken`]s.
///
/// Text segments become literal parts of the format string;
/// stand-in segments become `{}` with the corresponding
/// variable as a format argument.
fn format_expr_with_stand_ins(
    stand_ins: &[StandInToken<'_>],
    context: &Context,
) -> syn::Result<syn::Expr> {
    let mut format_str = String::new();
    let mut vars = vec![];

    for &stand_in in stand_ins {
        match stand_in {
            StandInToken::Text(t) => format_str.push_str(t),
            StandInToken::StandIn(p) => match context.stand_in(p) {
                Some(var) => {
                    format_str.push_str("{}");
                    vars.push(var);
                }
                None => {
                    return Err(syn::Error::new(Span::call_site(), UnboundVar(p.to_owned())));
                }
            },
        }
    }

    let format_args = vars.iter().map(|var| -> syn::Expr {
        let ident = &var.ident;
        match &var.ty {
            VarType::Option(inner) if inner.is_str() => {
                parse_quote!(#ident.as_deref().unwrap_or_default())
            }
            _ => parse_quote!(#ident),
        }
    });

    Ok(parse_quote!(format!(#format_str, #(#format_args),*)))
}

impl Lift for StringLiteralValue {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        let mut parts = vec![];
        for part in self.iter() {
            parts.push(unsplice!(part.lift(context)?));
        }
        let expr = match &*parts {
            [single] => {
                parse_quote!(#root::ruff::python_ast::StringLiteralValue::single(#single))
            }
            parts => parse_quote!(
                #root::ruff::python_ast::StringLiteralValue::concatenated(
                    vec![#(#parts),*]
                )
            ),
        };
        Ok(CodeFragment::Single(expr))
    }
}

impl Lift for StringLiteralFlags {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        Ok(CodeFragment::Single(parse_quote!(
            #root::ruff::python_ast::StringLiteralFlags::empty()
        )))
    }
}

// MARK: Bytes literals

impl Lift for BytesLiteral {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        let range = unsplice!(self.range.lift(context)?);
        let node_index = unsplice!(self.node_index.lift(context)?);
        let value = syn::LitByteStr::new(&self.value, Span::call_site());
        Ok(CodeFragment::Single(
            parse_quote!(#root::ruff::python_ast::BytesLiteral {
                range: #range,
                node_index: #node_index,
                value: (#value).to_vec().into_boxed_slice(),
                flags: #root::ruff::python_ast::BytesLiteralFlags::empty(),
            }),
        ))
    }
}

impl Lift for BytesLiteralValue {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        let mut parts = vec![];
        for part in self.iter() {
            parts.push(unsplice!(part.lift(context)?));
        }
        let expr = match &*parts {
            [single] => {
                parse_quote!(#root::ruff::python_ast::BytesLiteralValue::single(#single))
            }
            parts => parse_quote!(
                #root::ruff::python_ast::BytesLiteralValue::concatenated(
                    vec![#(#parts),*]
                )
            ),
        };
        Ok(CodeFragment::Single(expr))
    }
}

impl Lift for BytesLiteralFlags {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        Ok(CodeFragment::Single(parse_quote!(
            #root::ruff::python_ast::BytesLiteralFlags::empty()
        )))
    }
}

// MARK: Interpolated (f- and t-) strings

impl Lift for String {
    fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
        Ok(CodeFragment::Single(parse_quote!(String::from(#self))))
    }
}

impl_lift_for_unit_enum!(ConversionFlag, [None, Str, Ascii, Repr]);

impl_lift_for_struct!(DebugText, [leading, trailing]);

impl_lift_for_struct!(
    InterpolatedElement,
    [
        range,
        node_index,
        expression,
        debug_text,
        conversion,
        format_spec
    ]
);

impl_lift_for_struct!(InterpolatedStringLiteralElement, [range, node_index, value]);

impl_lift_for_struct!(InterpolatedStringFormatSpec, [range, node_index, elements]);

impl_lift_for_newtype_enum!(InterpolatedStringElement, [Interpolation, Literal]);

/// Lifts `InterpolatedStringElements`, a newtype around
/// `Vec<InterpolatedStringElement>`.
impl Lift for InterpolatedStringElements {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
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
            #root::ruff::python_ast::InterpolatedStringElements::from(items)
        }}))
    }
}

impl_lift_for_struct!(FString, [range, node_index, elements, flags]);

impl Lift for FStringFlags {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        Ok(CodeFragment::Single(parse_quote!(
            #root::ruff::python_ast::FStringFlags::empty()
        )))
    }
}

impl_lift_for_newtype_enum!(FStringPart, [Literal, FString]);

/// Lifts `FStringValue`, a newtype around `FStringValueInner`.
impl Lift for FStringValue {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        let parts = self.as_slice();
        let expr = match parts {
            [FStringPart::FString(f)] => {
                let inner = unsplice!(f.lift(context)?);
                parse_quote!(
                    #root::ruff::python_ast::FStringValue::single(#inner)
                )
            }
            parts => {
                let mut lifted = vec![];
                for part in parts {
                    lifted.push(unsplice!(part.lift(context)?));
                }
                parse_quote!(
                    #root::ruff::python_ast::FStringValue::concatenated(
                        vec![#(#lifted),*]
                    )
                )
            }
        };
        Ok(CodeFragment::Single(expr))
    }
}

impl_lift_for_struct!(TString, [range, node_index, elements, flags]);

impl Lift for TStringFlags {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        Ok(CodeFragment::Single(parse_quote!(
            #root::ruff::python_ast::TStringFlags::empty()
        )))
    }
}

/// Lifts `TStringValue`, a newtype around `TStringValueInner`.
impl Lift for TStringValue {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        let parts = self.as_slice();
        let mut lifted = vec![];
        for part in parts {
            lifted.push(unsplice!(part.lift(context)?));
        }
        let expr = match &*lifted {
            [single] => parse_quote!(
                #root::ruff::python_ast::TStringValue::single(#single)
            ),
            parts => parse_quote!(
                #root::ruff::python_ast::TStringValue::concatenated(vec![#(#parts),*])
            ),
        };
        Ok(CodeFragment::Single(expr))
    }
}

// MARK: Number types

impl Lift for ruff_python_ast::Int {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        let expr = match self.as_u64() {
            Some(small) => parse_quote!(
                #root::ruff::python_ast::Int::from(#small)
            ),
            None => {
                let s = self.to_string();
                parse_quote!(
                    #s.parse::<#root::ruff::python_ast::Int>().unwrap()
                )
            }
        };
        Ok(CodeFragment::Single(expr))
    }
}

impl Lift for Number {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        match self {
            Number::Int(i) => {
                let inner = unsplice!(i.lift(context)?);
                Ok(CodeFragment::Single(parse_quote!(
                    #root::ruff::python_ast::Number::Int(#inner)
                )))
            }
            Number::Float(f) => Ok(CodeFragment::Single(parse_quote!(
                #root::ruff::python_ast::Number::Float(#f)
            ))),
            Number::Complex { real, imag } => Ok(CodeFragment::Single(parse_quote!(
                #root::ruff::python_ast::Number::Complex {
                    real: #real,
                    imag: #imag,
                }
            ))),
        }
    }
}

// MARK: Singletons

impl_lift_for_unit_enum!(Singleton, [None, True, False]);
