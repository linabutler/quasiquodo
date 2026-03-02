use indoc::indoc;
use quasiquodo_py::py_quote;
use ruff_python_ast::*;
use ruff_python_codegen::{Generator, Indentation};
use ruff_source_file::LineEnding;

fn to_code_suite(suite: &Suite) -> String {
    // `Generator::unparse_suite` doesn't return the string buffer,
    // and there's no public method to take it. As a workaround,
    // accumulate and join formatted statements.
    suite
        .iter()
        .map(|s| Generator::new(&Indentation::default(), LineEnding::Lf).stmt(s))
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_code_stmt(stmt: &Stmt) -> String {
    Generator::new(&Indentation::default(), LineEnding::Lf).stmt(stmt)
}

// MARK: Static suites

#[test]
fn test_suite_single_statement() {
    let suite: Vec<Stmt> = py_quote!("pass" as Suite);
    assert_eq!(to_code_suite(&suite), "pass");
}

#[test]
fn test_suite_multiple_statements() {
    let suite: Vec<Stmt> = py_quote! {{"
        x = 1
        y = 2
        return x + y
    "} as Suite};
    assert_eq!(
        to_code_suite(&suite),
        indoc! {"
            x = 1
            y = 2
            return x + y"
        },
    );
}

// MARK: Suite variable substitution

#[test]
fn test_function_body_suite_variable() {
    let body: Vec<Stmt> = vec![
        py_quote!("y = x + 1" as Stmt),
        py_quote!("return y" as Stmt),
    ];
    let f: StmtFunctionDef = py_quote!({"
        def foo(x):
            #{body}
    "} as FunctionDef, body: Suite = body);
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
fn test_class_body_suite_variable() {
    let body: Vec<Stmt> = vec![
        py_quote!("x = 1" as Stmt),
        py_quote! {{"
            def greet(self):
                return self.x
        "} as Stmt},
    ];
    let c: StmtClassDef = py_quote!({"
        class Foo:
             #{body}
    "} as ClassDef, body: Suite = body);
    assert_eq!(
        to_code_stmt(&Stmt::ClassDef(c)),
        indoc! {"
            class Foo:
                x = 1

                def greet(self):
                    return self.x"
        },
    );
}

// MARK: Optional string in suite position

#[test]
fn test_suite_with_optional_str_some() {
    let desc: Option<&str> = Some("A thing.");
    let suite: Vec<Stmt> = py_quote!({"
        #{desc}
        x = 1
    "} as Suite, desc: Option<&str> = desc);
    assert_eq!(
        to_code_suite(&suite),
        indoc! {"
            'A thing.'
            x = 1"
        },
    );
}

#[test]
fn test_suite_with_optional_str_none() {
    let desc: Option<&str> = None;
    let suite: Vec<Stmt> = py_quote!({"
        #{desc}
        x = 1
    "} as Suite, desc: Option<&str> = desc);
    assert_eq!(to_code_suite(&suite), "x = 1");
}

#[test]
fn test_suite_with_optional_string_some() {
    let desc: Option<String> = Some("A thing.".to_owned());
    let suite: Vec<Stmt> = py_quote!({"
        #{desc}
        x = 1
    "} as Suite, desc: Option<String> = desc);
    assert_eq!(
        to_code_suite(&suite),
        indoc! {"
            'A thing.'
            x = 1"
        },
    );
}

// MARK: Suite with static prefix

#[test]
fn test_suite_variable_with_static_prefix() {
    let extra: Vec<Stmt> = vec![py_quote!("return y" as Stmt)];
    let f: StmtFunctionDef = py_quote!({"
        def foo(x):
            y = x + 1
            #{extra}
    "} as FunctionDef, extra: Suite = extra);
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo(x):
                y = x + 1
                return y"
        },
    );
}
