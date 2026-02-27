use indoc::indoc;
use quasiquodo_ts::ts_quote;
use swc_ecma_ast::*;
use swc_ecma_codegen::to_code;

// MARK: Static interfaces

#[test]
fn test_interface_with_members() {
    let ast = ts_quote!("export interface Pet { name: string; age?: number; }" as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export interface Pet {
                name: string;
                age?: number;
            }
        "},
    );
}

#[test]
fn test_interface_with_extends() {
    let ast = ts_quote!("export interface Dog extends Pet { breed: string; }" as ModuleItem);
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export interface Dog extends Pet {
                breed: string;
            }
        "},
    );
}

// MARK: Literal property names

#[test]
fn test_interface_prop_str_valid_ident() {
    let prop_name = "color";
    let elem = ts_quote!("#{name}: string" as TsTypeElement, name: &str = prop_name);
    assert_eq!(to_code(&elem), "color: string;");
}

#[test]
fn test_interface_prop_str_needs_quoting() {
    let prop_name = "background-color";
    let elem = ts_quote!("#{name}: string" as TsTypeElement, name: &str = prop_name);
    assert_eq!(to_code(&elem), r#""background-color": string;"#);
}

#[test]
fn test_interface_prop_string_valid_ident() {
    let prop_name = "color".to_owned();
    let elem = ts_quote!("#{name}: string" as TsTypeElement, name: String = prop_name);
    assert_eq!(to_code(&elem), "color: string;");
}

#[test]
fn test_interface_prop_string_needs_quoting() {
    let prop_name = "background-color".to_owned();
    let elem = ts_quote!("#{name}: string" as TsTypeElement, name: String = prop_name);
    assert_eq!(to_code(&elem), r#""background-color": string;"#);
}

// MARK: List splices

#[test]
fn test_interface_member_vec_splice() {
    let name: Ident = ts_quote!("Pet" as Ident);
    let members: Vec<TsTypeElement> = vec![
        ts_quote!("name: string" as TsTypeElement),
        ts_quote!("age?: number" as TsTypeElement),
    ];
    let ast = ts_quote!(
        "export interface #{N} { #{M}; }" as ModuleItem,
        N: Ident = name,
        M: Vec<TsTypeElement> = members
    );
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export interface Pet {
                name: string;
                age?: number;
            }
        "},
    );
}

#[test]
fn test_interface_extends_vec_splice() {
    let name: Ident = ts_quote!("Dog" as Ident);
    let extends: Vec<Ident> = vec![
        ts_quote!("Pet" as Ident),
        ts_quote!("Pet" as Ident),
        ts_quote!("Serializable" as Ident),
    ];
    let ast = ts_quote!(
        "export interface #{N} extends #{E} { breed: string; }" as ModuleItem,
        N: Ident = name,
        E: Vec<Ident> = extends
    );
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export interface Dog extends Pet, Pet, Serializable {
                breed: string;
            }
        "},
    );
}

// MARK: `Option` splices

#[test]
fn test_interface_member_option_splice_some() {
    let extra: Option<TsTypeElement> = Some(ts_quote!("age?: number" as TsTypeElement));
    let ast = ts_quote!(
        "export interface Pet { name: string; #{M}; }" as ModuleItem,
        M: Option<TsTypeElement> = extra
    );
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export interface Pet {
                name: string;
                age?: number;
            }
        "},
    );
}

#[test]
fn test_interface_member_option_splice_none() {
    let extra: Option<TsTypeElement> = None;
    let ast = ts_quote!(
        "export interface Pet { name: string; #{M}; }" as ModuleItem,
        M: Option<TsTypeElement> = extra
    );
    assert_eq!(
        to_code(&ast),
        indoc! {"
            export interface Pet {
                name: string;
            }
        "},
    );
}
