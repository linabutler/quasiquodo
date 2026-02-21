use indoc::indoc;
use quasiquodo::ts_quote;
use swc_ecma_ast::*;
use swc_ecma_codegen::to_code;

// MARK: Type alias declarations

#[test]
fn test_type_alias_string_keyword() {
    let ast = ts_quote!("export type T = string;" as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export type T = string;
        "},
    );
}

#[test]
fn test_type_alias_union() {
    let ast = ts_quote!(r#"export type Status = "active" | "inactive";"# as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            export type Status = "active" | "inactive";
        "#},
    );
}

#[test]
fn test_type_alias_array() {
    let ast = ts_quote!("export type T = string[];" as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export type T = string[];
        "},
    );
}

#[test]
fn test_type_alias_with_generics() {
    let ast = ts_quote!("export type T = Record<string, number>;" as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export type T = Record<string, number>;
        "},
    );
}

#[test]
fn test_generic_type_alias() {
    let ast = ts_quote!("export type Container<T> = { value: T; };" as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export type Container<T> = {
                value: T;
            };
        "},
    );
}

#[test]
fn test_type_alias_with_ident_variable() {
    let my_name: Ident = ts_quote!("MyType" as Ident);
    let ast = ts_quote!("export type @{Name} = string;" as ModuleItem, Name: Ident = my_name);
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export type MyType = string;
        "},
    );
}

// MARK: Import declarations

#[test]
fn test_import_named() {
    let ast = ts_quote!(r#"import type { Pet } from "./types";"# as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            import type { Pet } from "./types";
        "#},
    );
}

#[test]
fn test_import_star() {
    let ast = ts_quote!(r#"import * as types from "./types";"# as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            import * as types from "./types";
        "#},
    );
}

#[test]
fn test_import_specifiers_vec_splice() {
    let specs: Vec<ImportSpecifier> = vec![
        ts_quote!("Foo" as ImportSpecifier),
        ts_quote!("Bar" as ImportSpecifier),
    ];
    let ast = ts_quote!(
        r#"import type { @{Specs} } from "./types";"# as ModuleItem,
        Specs: Vec<ImportSpecifier> = specs
    );
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            import type { Foo, Bar } from "./types";
        "#},
    );
}

#[test]
fn test_import_fixed_plus_vec_splice() {
    let rest: Vec<ImportSpecifier> = vec![ts_quote!("Bar" as ImportSpecifier)];
    let ast = ts_quote!(
        r#"import { Foo, @{Rest} } from "./types";"# as ModuleItem,
        Rest: Vec<ImportSpecifier> = rest
    );
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            import { Foo, Bar } from "./types";
        "#},
    );
}

#[test]
fn test_import_vec_ident_splice() {
    let names: Vec<Ident> = vec![ts_quote!("Pet" as Ident), ts_quote!("Owner" as Ident)];
    let ast = ts_quote!(
        r#"import type { @{Names} } from "./types";"# as ModuleItem,
        Names: Vec<Ident> = names
    );
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            import type { Pet, Owner } from "./types";
        "#},
    );
}

#[test]
fn test_single_import_specifier_substitution() {
    let my_spec: ImportSpecifier = ts_quote!("Foo" as ImportSpecifier);
    let ast = ts_quote!(
        r#"import { @{s} } from "./types";"# as ModuleItem,
        s: ImportSpecifier = my_spec
    );
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            import { Foo } from "./types";
        "#},
    );
}

// MARK: Export declarations

#[test]
fn test_named_export_reexport() {
    let ast = ts_quote!(r#"export type { Pet } from "./types";"# as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            export type { Pet } from "./types";
        "#},
    );
}

#[test]
fn test_named_export_vec_export_specifier_splice() {
    let specs: Vec<ExportSpecifier> = vec![ts_quote!("Pet" as ExportSpecifier)];
    let ast = ts_quote!(
        r#"export type { @{Specs} } from "./types";"# as ModuleItem,
        Specs: Vec<ExportSpecifier> = specs
    );
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            export type { Pet } from "./types";
        "#},
    );
}

// MARK: `Decl` variable substitution

#[test]
fn test_decl_scalar_substitution_in_stmt() {
    let decl: Decl = ts_quote!("const x = 1;" as Decl);
    let stmt: Stmt = ts_quote!("{ @{d}; }" as Stmt, d: Decl = decl);
    assert_eq!(
        to_code(&stmt),
        indoc! {"{
            const x = 1;
        }"},
    );
}

#[test]
fn test_decl_scalar_substitution_in_module_item() {
    let decl: Decl = ts_quote!("const x = 1;" as Decl);
    let item: ModuleItem = ts_quote!("@{d}" as ModuleItem, d: Decl = decl);
    assert_eq!(
        to_code(&item),
        indoc! {"
            const x = 1;
        "},
    );
}

#[test]
fn test_decl_vec_splice_in_block() {
    let decls: Vec<Decl> = vec![
        ts_quote!("const x = 1;" as Decl),
        ts_quote!("const y = 2;" as Decl),
    ];
    let stmt: Stmt = ts_quote!("function f() { @{Decls}; }" as Stmt, Decls: Vec<Decl> = decls);
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            function f() {
                const x = 1;
                const y = 2;
            }
        "},
    );
}

#[test]
fn test_decl_option_splice_some() {
    let decl: Option<Decl> = Some(ts_quote!("const x = 1;" as Decl));
    let stmt: Stmt = ts_quote!("function f() { @{d}; }" as Stmt, d: Option<Decl> = decl);
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            function f() {
                const x = 1;
            }
        "},
    );
}

#[test]
fn test_decl_option_splice_none() {
    let decl: Option<Decl> = None;
    let stmt: Stmt = ts_quote!("function f() { @{d}; }" as Stmt, d: Option<Decl> = decl);
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            function f() {}
        "},
    );
}

#[test]
fn test_decl_export_substitution() {
    let decl: Decl = ts_quote!("interface Pet { name: string; }" as Decl);
    let item: ModuleItem = ts_quote!("export @{d}" as ModuleItem, d: Decl = decl);
    assert_eq!(
        to_code(&item),
        indoc! {"
            export interface Pet {
                name: string;
            }
        "},
    );
}

#[test]
fn test_decl_vec_splice_in_module_body() {
    let decls: Vec<Decl> = vec![
        ts_quote!("const x = 1;" as Decl),
        ts_quote!("const y = 2;" as Decl),
    ];
    let item: ModuleItem = ts_quote!(
        "namespace N { @{Decls}; }" as ModuleItem,
        Decls: Vec<Decl> = decls
    );
    assert_eq!(
        to_code(&item),
        indoc! {"
            namespace N {
                const x = 1;
                const y = 2;
            }
        "},
    );
}

// MARK: Enum declarations

#[test]
fn test_ts_enum() {
    let ast = ts_quote!(
        r#"export enum Status { Active = "active", Inactive = "inactive" }"# as ModuleItem
    );
    assert_eq!(
        to_code(&ast),
        indoc! {r#"
            export enum Status {
                Active = "active",
                Inactive = "inactive"
            }
        "#},
    );
}
