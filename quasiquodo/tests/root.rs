use quasiquodo::{ts::swc::ecma_ast::*, ts_quote};
use swc_ecma_codegen::to_code;

/// Verifies that `quasiquodo::ts_quote!` resolves the crate path
/// through `$crate::ts`, so that generated code references
/// `quasiquodo::ts::swc::...` instead of `quasiquodo_ts::swc::...`.
#[test]
fn test_crate_root_resolves_through_quasiquodo() {
    let ty: TsType = ts_quote!("string | null" as TsType);
    assert_eq!(to_code(&ty), "string | null");
}
