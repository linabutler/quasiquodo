use std::{collections::HashMap, fmt::Display};

use proc_macro2::Span;
use syn::parse_quote;

use super::input::{MacroInput, VarType, Variable};

/// Prepares variable bindings: emits `let` bindings
/// and builds the substitution [`Context`].
pub(crate) fn context(
    input: &MacroInput,
    stand_ins: HashMap<String, StandInData>,
) -> syn::Result<(Vec<syn::Stmt>, Context)> {
    use std::collections::hash_map::Entry;

    let mut bindings = vec![];
    let mut vars = HashMap::new();

    for Variable { name, ty, value } in &input.variables {
        match vars.entry(VarName(name.to_string())) {
            Entry::Occupied(entry) => {
                return Err(syn::Error::new(
                    name.span(),
                    format!("duplicate variable name `{}`", entry.key()),
                ));
            }
            Entry::Vacant(entry) => {
                // Emit `let quote_var_Name = <value>;`.
                let var_ident = syn::Ident::new(&format!("quote_var_{name}"), Span::mixed_site());
                bindings.push(parse_quote! {
                    let #var_ident = #value;
                });
                entry.insert(VarData {
                    ident: var_ident,
                    ty: ty.clone(),
                });
            }
        }
    }

    let context = Context {
        root: input.root.clone(),
        vars,
        stand_ins,
    };

    Ok((bindings, context))
}

/// Context for variable substitution during code generation.
pub(crate) struct Context {
    /// The resolved root crate path (e.g., `quasiquodo::py` or
    /// `quasiquodo_py`), injected by the declarative macro wrapper.
    root: syn::Path,
    /// The variables passed to the macro.
    vars: HashMap<VarName, VarData>,
    /// Maps stand-ins to variable names.
    stand_ins: HashMap<String, StandInData>,
}

impl Context {
    /// Returns the resolved root crate path for use in generated code.
    #[inline]
    pub fn root(&self) -> &syn::Path {
        &self.root
    }

    /// Looks up a variable by its stand-in (e.g., `__pyq_0__`).
    #[inline]
    pub fn stand_in(&self, value: &str) -> Option<&VarData> {
        let data = self.stand_ins.get(value)?;
        self.vars.get(&data.var)
    }
}

/// A variable name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct VarName(String);

impl VarName {
    #[inline]
    pub fn from_str(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl Display for VarName {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Data for a single substitution variable.
pub(crate) struct VarData {
    /// The identifier for the `let` binding in the generated block.
    pub ident: syn::Ident,
    /// The declared type of this variable.
    pub ty: VarType,
}

/// Data for a single stand-in in the preprocessed source.
pub(crate) struct StandInData {
    /// The variable name, corresponding to [`Context::vars`].
    pub var: VarName,
}

#[derive(Debug, thiserror::Error)]
#[error("variable `#{{{0}}}` not bound to a value")]
pub struct UnboundVar(pub String);
