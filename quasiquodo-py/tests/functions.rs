use indoc::indoc;
use quasiquodo_py::py_quote;
use ruff_python_ast::*;
use ruff_python_codegen::{Generator, Indentation};
use ruff_source_file::LineEnding;

fn to_code_stmt(stmt: &Stmt) -> String {
    Generator::new(&Indentation::default(), LineEnding::Lf).stmt(stmt)
}

// MARK: Function definitions

#[test]
fn test_function_def_simple() {
    let f: StmtFunctionDef = py_quote!(
        "def foo():
             pass
        " as FunctionDef
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo():
                pass"
        },
    );
}

#[test]
fn test_function_def_with_args() {
    let f: StmtFunctionDef = py_quote!(
        "def foo(x, y):
             return x + y
        " as FunctionDef
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo(x, y):
                return x + y"
        },
    );
}

#[test]
fn test_function_def_with_return_type() {
    let f: StmtFunctionDef = py_quote!(
        "def foo(x: int) -> int:
             return x
        " as FunctionDef
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo(x: int) -> int:
                return x"
        },
    );
}

#[test]
fn test_function_def_async() {
    let f: StmtFunctionDef = py_quote!(
        "async def foo():
             pass
        " as FunctionDef
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            async def foo():
                pass"
        },
    );
}

// MARK: Parameters

#[test]
fn test_parameter_simple() {
    let p: Parameter = py_quote!("x" as Parameter);
    assert_eq!(p.name.id.as_str(), "x");
}

#[test]
fn test_parameter_with_annotation() {
    let p: Parameter = py_quote!("x: int" as Parameter);
    assert_eq!(p.name.id.as_str(), "x");
    assert!(p.annotation.is_some());
}

#[test]
fn test_parameter_with_default() {
    let p: ParameterWithDefault = py_quote!("x=42" as ParameterWithDefault);
    assert_eq!(p.parameter.name.id.as_str(), "x");
    assert!(p.default.is_some());
}

// MARK: Vec splice

#[test]
fn test_function_def_vec_decorator_splice() {
    let decorators: Vec<Decorator> = vec![
        py_quote!("staticmethod" as Decorator),
        py_quote!("override" as Decorator),
    ];
    let f: StmtFunctionDef = py_quote!(
        "def foo():
             pass
        " as FunctionDef
    );
    let f = StmtFunctionDef {
        decorator_list: decorators,
        ..f
    };
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            @staticmethod
            @override
            def foo():
                pass"
        },
    );
}

#[test]
fn test_function_def_vec_parameter_with_default_splice() {
    let params: Vec<ParameterWithDefault> = vec![
        py_quote!("x=1" as ParameterWithDefault),
        py_quote!("y=2" as ParameterWithDefault),
    ];
    let f: StmtFunctionDef = py_quote!(
        "def foo(#{Params}):
             pass
        " as FunctionDef,
        Params: Vec<ParameterWithDefault> = params
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo(x=1, y=2):
                pass"
        },
    );
}

// MARK: Decorators

#[test]
fn test_decorator() {
    let d: Decorator = py_quote!("staticmethod" as Decorator);
    assert!(matches!(d.expression, Expr::Name(_)));
}

#[test]
fn test_decorator_with_args() {
    let d: Decorator = py_quote!(r#"app.route("/")"# as Decorator);
    assert!(matches!(d.expression, Expr::Call(_)));
}

// MARK: Keywords

#[test]
fn test_keyword() {
    let k: Keyword = py_quote!(r#"name="value""# as Keyword);
    let arg = k.arg.as_ref().expect("expected keyword arg");
    assert_eq!(arg.id.as_str(), "name");
}
