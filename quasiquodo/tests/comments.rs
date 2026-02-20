use indoc::indoc;
use quasiquodo::ts::Comments;
use quasiquodo::ts_quote;
use swc_ecma_ast::*;
use swc_ecma_codegen::to_code_with_comments;

// MARK: Static JSDoc on type elements

#[test]
fn test_comment_static_on_property() {
    let comments = Comments::new();
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** The pet's name. */ name: string" as TsTypeElement
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The pet's name. */ name: string;",
    );
}

#[test]
fn test_comment_static_on_method() {
    let comments = Comments::new();
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** Fetches the resource. */ get(id: string): Pet;" as TsTypeElement
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** Fetches the resource. */ get(id: string): Pet;",
    );
}

// MARK: Dynamic JSDoc with `LitStr`

#[test]
fn test_comment_dynamic_litstr() {
    let comments = Comments::new();
    let description = "The pet's name.";
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** $desc */ name: string" as TsTypeElement,
        desc: LitStr = description
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The pet's name. */ name: string;",
    );
}

#[test]
fn test_comment_dynamic_multiple_placeholders() {
    let comments = Comments::new();
    let noun = "name";
    let adj = "required";
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** The $noun is $adj. */ name: string" as TsTypeElement,
        noun: LitStr = noun,
        adj: LitStr = adj
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The name is required. */ name: string;",
    );
}

// MARK: JSDoc on declarations

#[test]
fn test_comment_on_type_alias() {
    let comments = Comments::new();
    let ast = ts_quote!(
        comments,
        "/** A string identifier. */ export type Id = string;" as ModuleItem
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            /** A string identifier. */ export type Id = string;
        "},
    );
}

#[test]
fn test_comment_on_interface() {
    let comments = Comments::new();
    let ast = ts_quote!(
        comments,
        "/** Represents a pet. */ export interface Pet { name: string; }" as ModuleItem
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            /** Represents a pet. */ export interface Pet {
                name: string;
            }
        "},
    );
}

// MARK: JSDoc on inner members

#[test]
fn test_comment_on_interface_member() {
    let comments = Comments::new();
    let ast = ts_quote!(
        comments,
        "export interface Pet { /** The pet's name. */ name: string; age?: number; }" as ModuleItem
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            export interface Pet {
                /** The pet's name. */ name: string;
                age?: number;
            }
        "},
    );
}

#[test]
fn test_comment_on_multiple_interface_members() {
    let comments = Comments::new();
    let ast = ts_quote!(
        comments,
        "export interface Pet { /** Name. */ name: string; /** Age. */ age?: number; }"
            as ModuleItem
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            export interface Pet {
                /** Name. */ name: string;
                /** Age. */ age?: number;
            }
        "},
    );
}

#[test]
fn test_comment_dynamic_on_interface_member() {
    let comments = Comments::new();
    let desc = "The pet's name.";
    let ast = ts_quote!(
        comments,
        "export interface Pet { /** $desc */ name: string; }" as ModuleItem,
        desc: LitStr = desc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            export interface Pet {
                /** The pet's name. */ name: string;
            }
        "},
    );
}

// MARK: Commented members spliced into interface

#[test]
fn test_comment_spliced_member() {
    let comments = Comments::new();
    let member: TsTypeElement = ts_quote!(
        comments,
        "/** The pet's name. */ name: string" as TsTypeElement
    );
    let ast = ts_quote!(
        "export interface Pet { $m; }" as ModuleItem,
        m: TsTypeElement = member
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            export interface Pet {
                /** The pet's name. */ name: string;
            }
        "},
    );
}

#[test]
fn test_comment_spliced_vec_members() {
    let comments = Comments::new();
    let members: Vec<TsTypeElement> = vec![
        ts_quote!(
            comments,
            "/** The pet's name. */ name: string" as TsTypeElement
        ),
        ts_quote!(
            comments,
            "/** The pet's age. */ age?: number" as TsTypeElement
        ),
    ];
    let ast = ts_quote!(
        "export interface Pet { $M; }" as ModuleItem,
        M: Vec<TsTypeElement> = members
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            export interface Pet {
                /** The pet's name. */ name: string;
                /** The pet's age. */ age?: number;
            }
        "},
    );
}

#[test]
fn test_comment_dynamic_spliced_member() {
    let comments = Comments::new();
    let desc = "The pet's name.";
    let member: TsTypeElement = ts_quote!(
        comments,
        "/** $desc */ name: string" as TsTypeElement,
        desc: LitStr = desc
    );
    let ast = ts_quote!(
        "export interface Pet { $m; }" as ModuleItem,
        m: TsTypeElement = member
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            export interface Pet {
                /** The pet's name. */ name: string;
            }
        "},
    );
}

// MARK: JSDoc escape sequences

#[test]
fn test_comment_escape_in_jsdoc() {
    // `$$docs` in a JSDoc comment is an escape for the literal text `$docs`.
    let comments = Comments::new();
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** See $$docs for more. */ name: string" as TsTypeElement
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** See $docs for more. */ name: string;",
    );
}

#[test]
fn test_comment_escape_and_variable_in_jsdoc() {
    // `$$ref` is a literal `$ref`; `$noun` is a variable substitution.
    let comments = Comments::new();
    let noun = "name";
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** The $$ref $noun. */ value: string" as TsTypeElement,
        noun: LitStr = noun
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The $ref name. */ value: string;",
    );
}

// MARK: No comment without parameter

#[test]
fn test_no_comment_without_parameter() {
    let desc = "The pet's name.";
    let elem: TsTypeElement = ts_quote!(
        "/** $desc */ name: string" as TsTypeElement,
        desc: LitStr = desc
    );
    assert_eq!(to_code_with_comments(None, &elem), "name: string;");
}
