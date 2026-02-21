use indoc::indoc;
use quasiquodo::ts_quote;
use swc_ecma_ast::*;
use swc_ecma_codegen::to_code;

// MARK: Static statements

#[test]
fn test_stmt_return() {
    let stmt: Stmt = ts_quote!("return 42;" as Stmt);
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            return 42;
        "},
    );
}

#[test]
fn test_stmt_variable_decl() {
    let stmt: Stmt = ts_quote!("const x = 1;" as Stmt);
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            const x = 1;
        "},
    );
}

// MARK: Function parameter splicing

#[test]
fn test_function_params_vec_splice() {
    let params: Vec<Param> = vec![ts_quote!("x: string" as Param)];
    let stmt: Stmt = ts_quote!(
        "function foo(@{Params}) {}" as Stmt,
        Params: Vec<Param> = params
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            function foo(x: string) {}
        "},
    );
}

#[test]
fn test_single_param_substitution() {
    let my_param: Param = ts_quote!("x: number" as Param);
    let stmt: Stmt = ts_quote!(
        "function foo(@{p}) {}" as Stmt,
        p: Param = my_param
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            function foo(x: number) {}
        "},
    );
}

// MARK: Block statement splicing

#[test]
fn test_block_stmts_vec_splice() {
    let body: Vec<Stmt> = vec![ts_quote!("return 2;" as Stmt)];
    let stmt: Stmt = ts_quote!(
        "function foo() { const x = 1; @{Body}; }" as Stmt,
        Body: Vec<Stmt> = body
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            function foo() {
                const x = 1;
                return 2;
            }
        "},
    );
}

// MARK: Class declarations

#[test]
fn test_class_body_vec_splice() {
    let members: Vec<ClassMember> = vec![ts_quote!("age: number;" as ClassMember)];
    let stmt: Stmt = ts_quote!(
        "class Foo { name: string; @{Members}; }" as Stmt,
        Members: Vec<ClassMember> = members
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            class Foo {
                name: string;
                age: number;
            }
        "},
    );
}

#[test]
fn test_single_class_member_substitution() {
    let member: ClassMember = ts_quote!("age: number;" as ClassMember);
    let stmt: Stmt = ts_quote!(
        "class Foo { name: string; @{m}; }" as Stmt,
        m: ClassMember = member
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            class Foo {
                name: string;
                age: number;
            }
        "},
    );
}

// MARK: Option splice

#[test]
fn test_block_stmts_option_splice_some() {
    let extra: Option<Stmt> = Some(ts_quote!("return 2;" as Stmt));
    let stmt: Stmt = ts_quote!(
        "function foo() { const x = 1; @{Body}; }" as Stmt,
        Body: Option<Stmt> = extra
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            function foo() {
                const x = 1;
                return 2;
            }
        "},
    );
}

#[test]
fn test_block_stmts_option_splice_none() {
    let extra: Option<Stmt> = None;
    let stmt: Stmt = ts_quote!(
        "function foo() { const x = 1; @{Body}; }" as Stmt,
        Body: Option<Stmt> = extra
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            function foo() {
                const x = 1;
            }
        "},
    );
}

#[test]
fn test_class_body_option_splice_some() {
    let member: Option<ClassMember> = Some(ts_quote!("age: number;" as ClassMember));
    let stmt: Stmt = ts_quote!(
        "class Foo { name: string; @{M}; }" as Stmt,
        M: Option<ClassMember> = member
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            class Foo {
                name: string;
                age: number;
            }
        "},
    );
}

#[test]
fn test_class_body_option_splice_none() {
    let member: Option<ClassMember> = None;
    let stmt: Stmt = ts_quote!(
        "class Foo { name: string; @{M}; }" as Stmt,
        M: Option<ClassMember> = member
    );
    assert_eq!(
        to_code(&stmt),
        indoc! {"
            class Foo {
                name: string;
            }
        "},
    );
}
