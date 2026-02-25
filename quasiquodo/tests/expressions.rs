use indoc::indoc;
use quasiquodo::ts_quote;
use swc_ecma_ast::*;
use swc_ecma_codegen::to_code;

// MARK: Static expressions

#[test]
fn test_expr_literal() {
    let expr: Expr = ts_quote!("42" as Expr);
    assert_eq!(to_code(&expr), "42");
}

#[test]
fn test_expr_call() {
    let expr: Expr = ts_quote!("foo()" as Expr);
    assert_eq!(to_code(&expr), "foo()");
}

#[test]
fn test_expr_member() {
    let expr: Expr = ts_quote!("this.name" as Expr);
    assert_eq!(to_code(&expr), "this.name");
}

// MARK: Variable substitution

#[test]
fn test_expr_variable() {
    let some_expr: Expr = ts_quote!("foo()" as Expr);
    let expr: Expr = ts_quote!("await #{val}" as Expr, val: Expr = some_expr);
    assert_eq!(to_code(&expr), "await foo()");
}

#[test]
fn test_num_in_expr_position() {
    let v = 42.0;
    let expr: Expr = ts_quote!("#{v}" as Expr, v: f64 = v);
    assert_eq!(to_code(&expr), "42");
}

#[test]
fn test_usize_in_expr_position() {
    let v: usize = 42;
    let expr: Expr = ts_quote!("#{v}" as Expr, v: usize = v);
    assert_eq!(to_code(&expr), "42");
}

#[test]
fn test_bool_in_expr_position() {
    let v = true;
    let expr: Expr = ts_quote!("#{v}" as Expr, v: bool = v);
    assert_eq!(to_code(&expr), "true");
}

// MARK: `&str` property name simplification

#[test]
fn test_member_prop_str_valid_ident() {
    let field_name = "name";
    let expr: Expr = ts_quote!("foo[#{bar}]" as Expr, bar: &str = field_name);
    assert_eq!(to_code(&expr), "foo.name");
}

#[test]
fn test_member_prop_str_needs_computed() {
    let field_name = "some-field";
    let expr: Expr = ts_quote!("foo[#{bar}]" as Expr, bar: &str = field_name);
    assert_eq!(to_code(&expr), r#"foo["some-field"]"#);
}

#[test]
fn test_member_prop_str_in_assignment() {
    let field_name = "name";
    let expr: Expr = ts_quote!(r#"foo[#{bar}] = "baz""# as Expr, bar: &str = field_name);
    assert_eq!(to_code(&expr), r#"foo.name = "baz""#);
}

#[test]
fn test_member_prop_str_in_assignment_needs_computed() {
    let field_name = "some-field";
    let expr: Expr = ts_quote!(r#"foo[#{bar}] = "baz""# as Expr, bar: &str = field_name);
    assert_eq!(to_code(&expr), r#"foo["some-field"] = "baz""#);
}

#[test]
fn test_object_prop_str_valid_ident() {
    let prop_name = "key";
    let expr: Expr = ts_quote!("({ [#{key}]: 1 })" as Expr, key: &str = prop_name);
    assert_eq!(
        to_code(&expr),
        indoc! {"({
            key: 1
        })"},
    );
}

#[test]
fn test_object_prop_str_needs_quoting() {
    let prop_name = "some-key";
    let expr: Expr = ts_quote!("({ [#{key}]: 1 })" as Expr, key: &str = prop_name);
    assert_eq!(
        to_code(&expr),
        indoc! {r#"({
            "some-key": 1
        })"#},
    );
}

// MARK: `BigInt` literals

#[test]
fn test_bigint_zero() {
    let expr: Expr = ts_quote!("0n" as Expr);
    assert_eq!(to_code(&expr), "0n");
}

#[test]
fn test_bigint_positive() {
    let expr: Expr = ts_quote!("123n" as Expr);
    assert_eq!(to_code(&expr), "123n");
}

#[test]
fn test_bigint_large() {
    let expr: Expr = ts_quote!("99999999999999999999999n" as Expr);
    assert_eq!(to_code(&expr), "99999999999999999999999n");
}

// MARK: List splices

#[test]
fn test_call_args_vec_expr_splice() {
    let args: Vec<Expr> = vec![ts_quote!("1" as Expr), ts_quote!("2" as Expr)];
    let expr: Expr = ts_quote!("foo(#{Args})" as Expr, Args: Vec<Expr> = args);
    assert_eq!(to_code(&expr), "foo(1, 2)");
}

#[test]
fn test_array_lit_vec_expr_splice() {
    let items: Vec<Expr> = vec![ts_quote!("1" as Expr), ts_quote!("2" as Expr)];
    let expr: Expr = ts_quote!("[#{Items}]" as Expr, Items: Vec<Expr> = items);
    assert_eq!(
        to_code(&expr),
        indoc! {"[
            1,
            2
        ]"},
    );
}
