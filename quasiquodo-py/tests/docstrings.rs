use indoc::indoc;
use quasiquodo_py::py_quote;
use ruff_python_ast::*;
use ruff_python_codegen::{Generator, Indentation};
use ruff_source_file::LineEnding;

fn to_code_stmt(stmt: &Stmt) -> String {
    Generator::new(&Indentation::default(), LineEnding::Lf).stmt(stmt)
}

// MARK: Static docstrings

#[test]
fn test_function_def_with_static_docstring() {
    let f = py_quote!(
        r#"def foo():
             "Hello world"
             pass
        "# as Stmt
    );
    assert_eq!(
        to_code_stmt(&f),
        indoc! {"
            def foo():
                'Hello world'
                pass"
        },
    );
}

#[test]
fn test_class_def_with_static_docstring() {
    let c: StmtClassDef = py_quote!(
        r#"class Foo:
             "A class."
             pass
        "# as ClassDef
    );
    assert_eq!(
        to_code_stmt(&Stmt::ClassDef(c)),
        indoc! {"
            class Foo:
                'A class.'
                pass"
        },
    );
}

// MARK: Docstring variable substitution

#[test]
fn test_function_def_with_docstring_variable() {
    let doc = "Hello world";
    let f: StmtFunctionDef = py_quote!(
        "def foo():
             #{doc}
             pass
        " as FunctionDef,
        doc: &str = doc
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo():
                'Hello world'
                pass"
        },
    );
}

#[test]
fn test_class_def_with_docstring_variable() {
    let doc = "A class.";
    let c: StmtClassDef = py_quote!(
        "class Foo:
             #{doc}
             pass
        " as ClassDef,
        doc: &str = doc
    );
    assert_eq!(
        to_code_stmt(&Stmt::ClassDef(c)),
        indoc! {"
            class Foo:
                'A class.'
                pass"
        },
    );
}

#[test]
fn test_function_def_with_string_docstring_variable() {
    let doc = String::from("Hello world");
    let f: StmtFunctionDef = py_quote!(
        "def foo():
             #{doc}
             pass
        " as FunctionDef,
        doc: String = doc
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo():
                'Hello world'
                pass"
        },
    );
}

// MARK: Multi-placeholder docstring interpolation

#[test]
fn test_docstring_with_two_placeholders() {
    let noun = "name";
    let adj = "required";
    let f: StmtFunctionDef = py_quote!(
        r#"def foo():
             """The #{noun} is #{adj}."""
             pass
        "# as FunctionDef,
        noun: &str = noun,
        adj: &str = adj
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo():
                'The name is required.'
                pass"
        },
    );
}

#[test]
fn test_docstring_with_embedded_str_variable() {
    let doc = "Hello world";
    let f: StmtFunctionDef = py_quote!(
        r#"def foo():
             """Summary: #{doc}"""
             pass
        "# as FunctionDef,
        doc: &str = doc
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo():
                'Summary: Hello world'
                pass"
        },
    );
}

#[test]
fn test_docstring_with_mixed_str_placeholders() {
    let noun = "pet";
    let doc = "a furry animal";
    let f: StmtFunctionDef = py_quote!(
        r#"def foo():
             """A #{noun} is #{doc}."""
             pass
        "# as FunctionDef,
        noun: &str = noun,
        doc: &str = doc
    );
    assert_eq!(
        to_code_stmt(&Stmt::FunctionDef(f)),
        indoc! {"
            def foo():
                'A pet is a furry animal.'
                pass"
        },
    );
}
