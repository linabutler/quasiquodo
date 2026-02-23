use indoc::indoc;
use quasiquodo_py::py_quote;
use ruff_python_ast::*;
use ruff_python_codegen::{Generator, Indentation};
use ruff_source_file::LineEnding;

fn to_code_stmt(stmt: &Stmt) -> String {
    Generator::new(&Indentation::default(), LineEnding::Lf).stmt(stmt)
}

// MARK: Static statements

#[test]
fn test_stmt_pass() {
    let stmt: Stmt = py_quote!("pass" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "pass");
}

#[test]
fn test_stmt_return() {
    let stmt: Stmt = py_quote!("return 42" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "return 42");
}

#[test]
fn test_stmt_return_none() {
    let stmt: Stmt = py_quote!("return" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "return");
}

#[test]
fn test_stmt_assign() {
    let stmt: Stmt = py_quote!("x = 1" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "x = 1");
}

#[test]
fn test_stmt_aug_assign() {
    let stmt: Stmt = py_quote!("x += 1" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "x += 1");
}

#[test]
fn test_stmt_ann_assign() {
    let stmt: Stmt = py_quote!("x: int = 1" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "x: int = 1");
}

#[test]
fn test_stmt_if() {
    let stmt: Stmt = py_quote!(
        "if x:
             pass
        " as Stmt
    );
    assert_eq!(
        to_code_stmt(&stmt),
        indoc! {"
            if x:
                pass"
        },
    );
}

#[test]
fn test_stmt_for() {
    let stmt: Stmt = py_quote!(
        "for x in xs:
             pass
        " as Stmt
    );
    assert_eq!(
        to_code_stmt(&stmt),
        indoc! {"
            for x in xs:
                pass"
        },
    );
}

#[test]
fn test_stmt_while() {
    let stmt: Stmt = py_quote!(
        "while True:
             pass
        " as Stmt
    );
    assert_eq!(
        to_code_stmt(&stmt),
        indoc! {"
            while True:
                pass"
        },
    );
}

#[test]
fn test_stmt_break() {
    let stmt: Stmt = py_quote!("break" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "break");
}

#[test]
fn test_stmt_continue() {
    let stmt: Stmt = py_quote!("continue" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "continue");
}

#[test]
fn test_stmt_raise() {
    let stmt: Stmt = py_quote!("raise ValueError()" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "raise ValueError()");
}

#[test]
fn test_stmt_assert() {
    let stmt: Stmt = py_quote!("assert x > 0" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "assert x > 0");
}

#[test]
fn test_stmt_delete() {
    let stmt: Stmt = py_quote!("del x" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "del x");
}

#[test]
fn test_stmt_global() {
    let stmt: Stmt = py_quote!("global x" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "global x");
}

// MARK: Variable substitution

#[test]
fn test_stmt_variable() {
    let inner: Stmt = py_quote!("pass" as Stmt);
    let stmt: Stmt = py_quote!("#{s}" as Stmt, s: Stmt = inner);
    assert_eq!(to_code_stmt(&stmt), "pass");
}

#[test]
fn test_stmt_expr_with_variable() {
    let val: Expr = py_quote!("42" as Expr);
    let stmt: Stmt = py_quote!("return #{v}" as Stmt, v: Expr = val);
    assert_eq!(to_code_stmt(&stmt), "return 42");
}

// MARK: Vec splice

#[test]
fn test_function_body_vec_stmt_splice() {
    let body: Vec<Stmt> = vec![
        py_quote!("y = x + 1" as Stmt),
        py_quote!("return y" as Stmt),
    ];
    let f: StmtFunctionDef = py_quote!(
        "def foo(x):
             pass
             #{Body}
        " as FunctionDef,
        Body: Vec<Stmt> = body
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo(x):
                pass
                y = x + 1
                return y"
        },
    );
}

#[test]
fn test_class_body_vec_stmt_splice() {
    let methods: Vec<Stmt> = vec![py_quote!(
        "def greet(self):
             return self.name
        " as Stmt
    )];
    let c: StmtClassDef = py_quote!(
        "class Foo:
             name = None
             #{Methods}
        " as ClassDef,
        Methods: Vec<Stmt> = methods
    );
    assert_eq!(
        to_code_stmt(&Stmt::ClassDef(c)),
        indoc! {"
            class Foo:
                name = None

                def greet(self):
                    return self.name"
        },
    );
}

// MARK: Option splice

#[test]
fn test_function_body_option_stmt_splice_some() {
    let extra: Option<Stmt> = Some(py_quote!("return y" as Stmt));
    let f: StmtFunctionDef = py_quote!(
        "def foo(x):
             y = x + 1
             #{Extra}
        " as FunctionDef,
        Extra: Option<Stmt> = extra
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo(x):
                y = x + 1
                return y"
        },
    );
}

#[test]
fn test_function_body_option_stmt_splice_none() {
    let extra: Option<Stmt> = None;
    let f: StmtFunctionDef = py_quote!(
        "def foo(x):
             y = x + 1
             #{Extra}
        " as FunctionDef,
        Extra: Option<Stmt> = extra
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo(x):
                y = x + 1"
        },
    );
}
