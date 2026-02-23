use quasiquodo_py::py_quote;
use ruff_python_ast::*;
use ruff_python_codegen::{Generator, Indentation};
use ruff_source_file::LineEnding;

fn to_code_stmt(stmt: &Stmt) -> String {
    Generator::new(&Indentation::default(), LineEnding::Lf).stmt(stmt)
}

// MARK: Import statements

#[test]
fn test_import() {
    let stmt: Stmt = py_quote!("import os" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "import os");
}

#[test]
fn test_import_from() {
    let stmt: Stmt = py_quote!("from os import path" as Stmt);
    assert_eq!(to_code_stmt(&stmt), "from os import path");
}

#[test]
fn test_import_from_output_kind() {
    let imp: StmtImportFrom = py_quote!("from os import path" as ImportFrom);
    assert_eq!(imp.module.as_ref().unwrap().id.as_str(), "os");
    assert_eq!(imp.names.len(), 1);
    assert_eq!(imp.names[0].name.id.as_str(), "path");
}

// MARK: Alias

#[test]
fn test_alias_simple() {
    let a: Alias = py_quote!("path" as Alias);
    assert_eq!(a.name.id.as_str(), "path");
    assert!(a.asname.is_none());
}

#[test]
fn test_alias_with_asname() {
    let a: Alias = py_quote!("path as p" as Alias);
    assert_eq!(a.name.id.as_str(), "path");
    assert_eq!(a.asname.as_ref().unwrap().id.as_str(), "p");
}
