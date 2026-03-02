use quasiquodo_py::py_quote;
use ruff_python_ast::*;
use ruff_python_codegen::{Generator, Indentation};
use ruff_source_file::LineEnding;

fn to_code_expr(expr: &Expr) -> String {
    Generator::new(&Indentation::default(), LineEnding::Lf).expr(expr)
}

// MARK: Static expressions

#[test]
fn test_expr_number_literal() {
    let expr: Expr = py_quote!("42" as Expr);
    assert_eq!(to_code_expr(&expr), "42");
}

#[test]
fn test_expr_string_literal() {
    let expr: Expr = py_quote!(r#""hello""# as Expr);
    assert_eq!(to_code_expr(&expr), "'hello'");
}

#[test]
fn test_expr_bool_literal() {
    let expr: Expr = py_quote!("True" as Expr);
    assert_eq!(to_code_expr(&expr), "True");
}

#[test]
fn test_expr_none_literal() {
    let expr: Expr = py_quote!("None" as Expr);
    assert_eq!(to_code_expr(&expr), "None");
}

#[test]
fn test_expr_call() {
    let expr: Expr = py_quote!("foo()" as Expr);
    assert_eq!(to_code_expr(&expr), "foo()");
}

#[test]
fn test_expr_call_with_args() {
    let expr: Expr = py_quote!("foo(1, 2, 3)" as Expr);
    assert_eq!(to_code_expr(&expr), "foo(1, 2, 3)");
}

#[test]
fn test_expr_binary_op() {
    let expr: Expr = py_quote!("x + 1" as Expr);
    assert_eq!(to_code_expr(&expr), "x + 1");
}

#[test]
fn test_expr_compare() {
    let expr: Expr = py_quote!("x > 0" as Expr);
    assert_eq!(to_code_expr(&expr), "x > 0");
}

#[test]
fn test_expr_attribute() {
    let expr: Expr = py_quote!("foo.bar" as Expr);
    assert_eq!(to_code_expr(&expr), "foo.bar");
}

#[test]
fn test_expr_subscript() {
    let expr: Expr = py_quote!("foo[0]" as Expr);
    assert_eq!(to_code_expr(&expr), "foo[0]");
}

#[test]
fn test_expr_list() {
    let expr: Expr = py_quote!("[1, 2, 3]" as Expr);
    assert_eq!(to_code_expr(&expr), "[1, 2, 3]");
}

#[test]
fn test_expr_tuple() {
    let expr: Expr = py_quote!("(1, 2, 3)" as Expr);
    // Ruff's codegen omits parentheses for top-level tuple expressions.
    assert_eq!(to_code_expr(&expr), "1, 2, 3");
}

#[test]
fn test_expr_dict() {
    let expr: Expr = py_quote!(r#"{"a": 1, "b": 2}"# as Expr);
    assert_eq!(to_code_expr(&expr), "{'a': 1, 'b': 2}");
}

#[test]
fn test_expr_unary_op() {
    let expr: Expr = py_quote!("not x" as Expr);
    assert_eq!(to_code_expr(&expr), "not x");
}

#[test]
fn test_expr_lambda() {
    let expr: Expr = py_quote!("lambda x: x + 1" as Expr);
    assert_eq!(to_code_expr(&expr), "lambda x: x + 1");
}

#[test]
fn test_expr_if() {
    let expr: Expr = py_quote!("x if True else y" as Expr);
    assert_eq!(to_code_expr(&expr), "x if True else y");
}

// MARK: Variable substitution

#[test]
fn test_expr_variable() {
    let inner: Expr = py_quote!("foo()" as Expr);
    let expr: Expr = py_quote!("await #{val}" as Expr, val: Expr = inner);
    assert_eq!(to_code_expr(&expr), "await foo()");
}

#[test]
fn test_expr_variable_in_call() {
    let arg: Expr = py_quote!("42" as Expr);
    let expr: Expr = py_quote!("foo(#{x})" as Expr, x: Expr = arg);
    assert_eq!(to_code_expr(&expr), "foo(42)");
}

#[test]
fn test_lit_num_in_expr_position() {
    let v = 42.0;
    let expr: Expr = py_quote!("#{v}" as Expr, v: f64 = v);
    assert_eq!(to_code_expr(&expr), "42.0");
}

#[test]
fn test_lit_bool_in_expr_position() {
    let v = true;
    let expr: Expr = py_quote!("#{v}" as Expr, v: bool = v);
    assert_eq!(to_code_expr(&expr), "True");
}

#[test]
fn test_lit_str_in_expr_position() {
    let v = "hello";
    let expr: Expr = py_quote!("#{v}" as Expr, v: &str = v);
    assert_eq!(to_code_expr(&expr), "'hello'");
}

// MARK: Integer variables

#[test]
fn test_lit_u8_in_expr_position() {
    let v: u8 = 7;
    let expr: Expr = py_quote!("#{v}" as Expr, v: u8 = v);
    assert_eq!(to_code_expr(&expr), "7");
}

#[test]
fn test_lit_u16_in_expr_position() {
    let v: u16 = 256;
    let expr: Expr = py_quote!("#{v}" as Expr, v: u16 = v);
    assert_eq!(to_code_expr(&expr), "256");
}

#[test]
fn test_lit_u32_in_expr_position() {
    let v: u32 = 100_000;
    let expr: Expr = py_quote!("#{v}" as Expr, v: u32 = v);
    assert_eq!(to_code_expr(&expr), "100000");
}

#[test]
fn test_lit_u64_in_expr_position() {
    let v: u64 = 1_000_000_000;
    let expr: Expr = py_quote!("#{v}" as Expr, v: u64 = v);
    assert_eq!(to_code_expr(&expr), "1000000000");
}

// MARK: String variables

#[test]
fn test_lit_string_in_expr_position() {
    let v = String::from("hello");
    let expr: Expr = py_quote!("#{v}" as Expr, v: String = v);
    assert_eq!(to_code_expr(&expr), "'hello'");
}

#[test]
fn test_lit_box_str_in_expr_position() {
    let v: Box<str> = Box::from("hello");
    let expr: Expr = py_quote!("#{v}" as Expr, v: Box<str> = v);
    assert_eq!(to_code_expr(&expr), "'hello'");
}

#[test]
fn test_litstr_placeholder_in_regular_string_not_interpolated() {
    let v = "bar";
    let expr: Expr = py_quote!(r#""foo #{v} baz""# as Expr, v: &str = v);
    assert_eq!(to_code_expr(&expr), "'foo #{v} baz'");
}

// MARK: Strings in iterable positions

#[test]
fn test_optional_str_some_in_list() {
    let item: Option<&str> = Some("hello");
    let expr: Expr = py_quote!("[1, #{item}]" as Expr, item: Option<&str> = item);
    assert_eq!(to_code_expr(&expr), "[1, 'hello']");
}

#[test]
fn test_optional_str_none_in_list() {
    let item: Option<&str> = None;
    let expr: Expr = py_quote!("[1, #{item}]" as Expr, item: Option<&str> = item);
    assert_eq!(to_code_expr(&expr), "[1]");
}

#[test]
fn test_vec_str_in_list() {
    let items: Vec<&str> = vec!["a", "b"];
    let expr: Expr = py_quote!("[#{items}]" as Expr, items: Vec<&str> = items);
    assert_eq!(to_code_expr(&expr), "['a', 'b']");
}

// MARK: Identifier substitution

#[test]
fn test_identifier_in_name_position() {
    let name = Identifier::new("my_func", ruff_text_size::TextRange::default());
    let expr: Expr = py_quote!("#{name}()" as Expr, name: Identifier = name);
    assert_eq!(to_code_expr(&expr), "my_func()");
}

// MARK: Vec splice

#[test]
fn test_call_args_vec_expr_splice() {
    let args: Vec<Expr> = vec![py_quote!("1" as Expr), py_quote!("2" as Expr)];
    let expr: Expr = py_quote!("foo(#{Args})" as Expr, Args: Vec<Expr> = args);
    assert_eq!(to_code_expr(&expr), "foo(1, 2)");
}

#[test]
fn test_list_vec_expr_splice() {
    let items: Vec<Expr> = vec![py_quote!("1" as Expr), py_quote!("2" as Expr)];
    let expr: Expr = py_quote!("[#{Items}]" as Expr, Items: Vec<Expr> = items);
    assert_eq!(to_code_expr(&expr), "[1, 2]");
}

// MARK: f-strings

#[test]
fn test_fstring_static() {
    let expr: Expr = py_quote!(r#"f"hello {name}""# as Expr);
    assert_eq!(to_code_expr(&expr), "f'hello {name}'");
}

#[test]
fn test_fstring_with_conversion() {
    let expr: Expr = py_quote!(r#"f"{x!r}""# as Expr);
    assert_eq!(to_code_expr(&expr), "f'{x!r}'");
}

#[test]
fn test_fstring_with_format_spec() {
    let expr: Expr = py_quote!(r#"f"{x:.2f}""# as Expr);
    assert_eq!(to_code_expr(&expr), "f'{x:.2f}'");
}

#[test]
fn test_fstring_with_variable_in_hole() {
    let v: Expr = py_quote!("42" as Expr);
    let expr: Expr = py_quote!(r#"f"result: {#{v}}""# as Expr, v: Expr = v);
    assert_eq!(to_code_expr(&expr), "f'result: {42}'");
}

#[test]
fn test_fstring_multiple_holes() {
    let expr: Expr = py_quote!(r#"f"{x} + {y} = {z}""# as Expr);
    assert_eq!(to_code_expr(&expr), "f'{x} + {y} = {z}'");
}

// MARK: t-strings

#[test]
fn test_tstring_static() {
    let expr: Expr = py_quote!(r#"t"hello {name}""# as Expr);
    assert_eq!(to_code_expr(&expr), "t'hello {name}'");
}
