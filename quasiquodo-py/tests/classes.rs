use indoc::indoc;
use quasiquodo_py::py_quote;
use ruff_python_ast::*;
use ruff_python_codegen::{Generator, Indentation};
use ruff_source_file::LineEnding;

fn to_code_stmt(stmt: &Stmt) -> String {
    Generator::new(&Indentation::default(), LineEnding::Lf).stmt(stmt)
}

// MARK: Class definitions

#[test]
fn test_class_def_simple() {
    let c: StmtClassDef = py_quote!(
        "class Foo:
             pass
        " as ClassDef
    );
    assert_eq!(
        to_code_stmt(&Stmt::ClassDef(c)),
        indoc! {"
            class Foo:
                pass"
        },
    );
}

#[test]
fn test_class_def_with_base() {
    let c: StmtClassDef = py_quote!(
        "class Foo(Bar):
             pass
        " as ClassDef
    );
    assert_eq!(
        to_code_stmt(&Stmt::ClassDef(c)),
        indoc! {"
            class Foo(Bar):
                pass"
        },
    );
}

#[test]
fn test_class_def_with_body() {
    let c: StmtClassDef = py_quote!(
        "class Foo:
             x = 1
             y = 2
        " as ClassDef
    );
    assert_eq!(
        to_code_stmt(&Stmt::ClassDef(c)),
        indoc! {"
            class Foo:
                x = 1
                y = 2"
        },
    );
}
