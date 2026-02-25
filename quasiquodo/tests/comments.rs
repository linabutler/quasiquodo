use indoc::indoc;
use quasiquodo::ts::{Comments, JsDoc};
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

// MARK: Dynamic JSDoc with `&str`

#[test]
fn test_comment_dynamic_str() {
    let comments = Comments::new();
    let description = "The pet's name.";
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** #{desc} */ name: string" as TsTypeElement,
        desc: &str = description
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
        "/** The #{noun} is #{adj}. */ name: string" as TsTypeElement,
        noun: &str = noun,
        adj: &str = adj
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The name is required. */ name: string;",
    );
}

// MARK: Dynamic JSDoc with `String`

#[test]
fn test_comment_dynamic_string() {
    let comments = Comments::new();
    let description = "The pet's name.".to_owned();
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** #{desc} */ name: string" as TsTypeElement,
        desc: String = description
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The pet's name. */ name: string;",
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
        "export interface Pet { /** #{desc} */ name: string; }" as ModuleItem,
        desc: &str = desc
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
        "export interface Pet { #{m}; }" as ModuleItem,
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
        "export interface Pet { #{M}; }" as ModuleItem,
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
        "/** #{desc} */ name: string" as TsTypeElement,
        desc: &str = desc
    );
    let ast = ts_quote!(
        "export interface Pet { #{m}; }" as ModuleItem,
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

// MARK: `JsDoc` variable

#[test]
fn test_comment_jsdoc_variable_on_type_element() {
    let comments = Comments::new();
    let doc = JsDoc::new("The pet's name.");
    let elem: TsTypeElement = ts_quote!(
        comments,
        "#{doc} name: string" as TsTypeElement,
        doc: JsDoc = doc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The pet's name. */ name: string;",
    );
}

#[test]
fn test_comment_jsdoc_variable_on_interface_member() {
    let comments = Comments::new();
    let doc = JsDoc::new("The pet's name.");
    let ast = ts_quote!(
        comments,
        "export interface Pet { #{doc} name: string; }" as ModuleItem,
        doc: JsDoc = doc
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
fn test_comment_jsdoc_variable_on_class_member() {
    let comments = Comments::new();
    let doc = JsDoc::new("The pet's name.");
    let stmt: Stmt = ts_quote!(
        comments,
        "class Pet { #{doc} name: string; }" as Stmt,
        doc: JsDoc = doc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &stmt),
        indoc! {"
            class Pet {
                /** The pet's name. */ name: string;
            }
        "},
    );
}

#[test]
fn test_comment_jsdoc_variable_spliced_member() {
    let comments = Comments::new();
    let doc = JsDoc::new("The pet's name.");
    let member: TsTypeElement = ts_quote!(
        comments,
        "#{doc} name: string" as TsTypeElement,
        doc: JsDoc = doc
    );
    let ast = ts_quote!(
        "export interface Pet { #{m}; }" as ModuleItem,
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

// MARK: `Option<JsDoc>` variable

#[test]
fn test_comment_option_jsdoc_some() {
    let comments = Comments::new();
    let doc = Some(JsDoc::new("The pet's name."));
    let elem: TsTypeElement = ts_quote!(
        comments,
        "#{doc} name: string" as TsTypeElement,
        doc: Option<JsDoc> = doc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The pet's name. */ name: string;",
    );
}

#[test]
fn test_comment_option_jsdoc_none() {
    let comments = Comments::new();
    let doc: Option<JsDoc> = None;
    let elem: TsTypeElement = ts_quote!(
        comments,
        "#{doc} name: string" as TsTypeElement,
        doc: Option<JsDoc> = doc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "name: string;",
    );
}

// MARK: `Option<&str>` variable

#[test]
fn test_comment_option_str_some() {
    let comments = Comments::new();
    let desc: Option<&str> = Some("The pet's name.");
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** #{desc} */ name: string" as TsTypeElement,
        desc: Option<&str> = desc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The pet's name. */ name: string;",
    );
}

#[test]
fn test_comment_option_str_none() {
    let comments = Comments::new();
    let desc: Option<&str> = None;
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** #{desc} */ name: string" as TsTypeElement,
        desc: Option<&str> = desc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "name: string;",
    );
}

// MARK: `Option<String>` variable

#[test]
fn test_comment_option_string_some() {
    let comments = Comments::new();
    let desc: Option<String> = Some("The pet's name.".to_owned());
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** #{desc} */ name: string" as TsTypeElement,
        desc: Option<String> = desc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** The pet's name. */ name: string;",
    );
}

#[test]
fn test_comment_option_string_none() {
    let comments = Comments::new();
    let desc: Option<String> = None;
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** #{desc} */ name: string" as TsTypeElement,
        desc: Option<String> = desc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "name: string;",
    );
}

// MARK: `JsDoc` embedded in comment

#[test]
fn test_comment_jsdoc_embedded_with_text() {
    let comments = Comments::new();
    let doc = JsDoc::new("a pet");
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** This is #{doc}. */ name: string" as TsTypeElement,
        doc: JsDoc = doc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** This is a pet. */ name: string;",
    );
}

#[test]
fn test_comment_option_jsdoc_embedded_some() {
    let comments = Comments::new();
    let doc: Option<JsDoc> = Some(JsDoc::new("a pet"));
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** This is #{doc}. */ name: string" as TsTypeElement,
        doc: Option<JsDoc> = doc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** This is a pet. */ name: string;",
    );
}

#[test]
fn test_comment_option_jsdoc_embedded_none() {
    let comments = Comments::new();
    let doc: Option<JsDoc> = None;
    let elem: TsTypeElement = ts_quote!(
        comments,
        "/** This is #{doc}. */ name: string" as TsTypeElement,
        doc: Option<JsDoc> = doc
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &elem),
        "/** This is . */ name: string;",
    );
}

// MARK: Multi-level JSDoc nesting

#[test]
fn test_comment_jsdoc_survives_multi_level_splice() {
    // Level 1: attach a `JsDoc` to a `ClassMember`.
    // Level 2: splice that member into a class `Stmt`.
    // Level 3: splice the class into a block `Stmt`.
    // The comment should survive all three levels.
    let comments = Comments::new();
    let doc = JsDoc::new("The pet's name.");
    let member: ClassMember = ts_quote!(
        comments,
        "#{doc} name: string" as ClassMember,
        doc: JsDoc = doc
    );
    let class: Stmt = ts_quote!(
        "class Pet { #{m} }" as Stmt,
        m: ClassMember = member
    );
    let block: Stmt = ts_quote!(
        "{ #{s} }" as Stmt,
        s: Stmt = class
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &block),
        indoc! {"{
            class Pet {
                /** The pet's name. */ name: string;
            }
        }"},
    );
}

#[test]
fn test_comment_jsdoc_coexists_with_outer_comment() {
    // Inner `JsDoc` on a member, outer static comment on the interface.
    // Both should appear in the output.
    let comments = Comments::new();
    let doc = JsDoc::new("The name.");
    let member: TsTypeElement = ts_quote!(
        comments,
        "#{doc} name: string" as TsTypeElement,
        doc: JsDoc = doc
    );
    let ast = ts_quote!(
        comments,
        "/** A pet. */ export interface Pet { #{m}; }" as ModuleItem,
        m: TsTypeElement = member
    );
    assert_eq!(
        to_code_with_comments(Some(&*comments), &ast),
        indoc! {"
            /** A pet. */ export interface Pet {
                /** The name. */ name: string;
            }
        "},
    );
}

// MARK: No comment without parameter

#[test]
fn test_no_comment_without_parameter() {
    let desc = "The pet's name.";
    let elem: TsTypeElement = ts_quote!(
        "/** #{desc} */ name: string" as TsTypeElement,
        desc: &str = desc
    );
    assert_eq!(to_code_with_comments(None, &elem), "name: string;");
}
