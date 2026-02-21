use indoc::indoc;
use quasiquodo::ts_quote;
use swc_ecma_ast::*;
use swc_ecma_codegen::to_code;

// MARK: Keyword types

#[test]
fn test_type_keyword() {
    let ty: TsType = ts_quote!("string" as TsType);
    assert_eq!(to_code(&ty), "string");
}

#[test]
fn test_type_union() {
    let ty: TsType = ts_quote!("string | null" as TsType);
    assert_eq!(to_code(&ty), "string | null");
}

#[test]
fn test_type_array() {
    let ty: TsType = ts_quote!("string[]" as TsType);
    assert_eq!(to_code(&ty), "string[]");
}

#[test]
fn test_type_object_literal() {
    let ty: TsType = ts_quote!("{ name: string; age?: number }" as TsType);
    assert_eq!(
        to_code(&ty),
        indoc! {"{
            name: string;
            age?: number;
        }"},
    );
}

// MARK: Literal types

#[test]
fn test_lit_type_string() {
    let ty: TsType = ts_quote!(r#""hello""# as TsType);
    assert_eq!(to_code(&ty), r#""hello""#);
}

#[test]
fn test_lit_type_number() {
    let ty: TsType = ts_quote!("42" as TsType);
    assert_eq!(to_code(&ty), "42");
}

#[test]
fn test_lit_type_boolean_true() {
    let ty: TsType = ts_quote!("true" as TsType);
    assert_eq!(to_code(&ty), "true");
}

#[test]
fn test_lit_type_boolean_false() {
    let ty: TsType = ts_quote!("false" as TsType);
    assert_eq!(to_code(&ty), "false");
}

#[test]
fn test_lit_type_bigint() {
    let ty: TsType = ts_quote!("123n" as TsType);
    assert_eq!(to_code(&ty), "123n");
}

#[test]
fn test_lit_type_template() {
    let ty: TsType = ts_quote!(r#"`hello ${world}`"# as TsType);
    assert_eq!(to_code(&ty), "`hello ${world}`");
}

// MARK: Type references

#[test]
fn test_type_ref_simple() {
    let ty: TsType = ts_quote!("Pet" as TsType);
    assert_eq!(to_code(&ty), "Pet");
}

#[test]
fn test_type_ref_qualified() {
    let ty: TsType = ts_quote!("Order.Status" as TsType);
    assert_eq!(to_code(&ty), "Order.Status");
}

#[test]
fn test_type_ref_deeply_qualified() {
    let ty: TsType = ts_quote!("API.Order.Status" as TsType);
    assert_eq!(to_code(&ty), "API.Order.Status");
}

#[test]
fn test_type_ref_with_type_params() {
    let ty: TsType = ts_quote!("Array<string>" as TsType);
    assert_eq!(to_code(&ty), "Array<string>");
}

// MARK: Variable substitution

#[test]
fn test_ts_type_variable() {
    let some_type: TsType = ts_quote!("string" as TsType);
    let ty: TsType = ts_quote!("@{T} | null" as TsType, T: TsType = some_type);
    assert_eq!(to_code(&ty), "string | null");
}

#[test]
fn test_lit_str_in_type_position() {
    let v = "hello";
    let ty: TsType = ts_quote!("@{v}" as TsType, v: LitStr = v);
    assert_eq!(to_code(&ty), r#""hello""#);
}

#[test]
fn test_lit_num_in_type_position() {
    let v = 42.0;
    let ty: TsType = ts_quote!("@{v}" as TsType, v: LitNum = v);
    assert_eq!(to_code(&ty), "42");
}

#[test]
fn test_lit_bool_in_type_position() {
    let v = true;
    let ty: TsType = ts_quote!("@{v}" as TsType, v: LitBool = v);
    assert_eq!(to_code(&ty), "true");
}

#[test]
fn test_lit_str_in_union() {
    let active = "Active";
    let inactive = "Inactive";
    let ty: TsType = ts_quote!(
        "@{active} | @{inactive}" as TsType,
        active: LitStr = active,
        inactive: LitStr = inactive
    );
    assert_eq!(to_code(&ty), r#""Active" | "Inactive""#);
}

// MARK: List splices

#[test]
fn test_union_fixed_plus_vec_splice() {
    let extra: Vec<Box<TsType>> = vec![
        Box::new(ts_quote!("number" as TsType)),
        Box::new(ts_quote!("boolean" as TsType)),
    ];
    let ty: TsType = ts_quote!(
        "string | @{Extra}" as TsType,
        Extra: Vec<Box<TsType>> = extra
    );
    assert_eq!(to_code(&ty), "string | number | boolean");
}

#[test]
fn test_union_single_plus_vec_splice() {
    let base: TsType = ts_quote!("string" as TsType);
    let rest: Vec<Box<TsType>> = vec![Box::new(ts_quote!("null" as TsType))];
    let ty: TsType = ts_quote!(
        "@{T} | @{Rest}" as TsType,
        T: TsType = base,
        Rest: Vec<Box<TsType>> = rest
    );
    assert_eq!(to_code(&ty), "string | null");
}

#[test]
fn test_intersection_vec_splice() {
    let base: TsType = ts_quote!("Base" as TsType);
    let mixins: Vec<Box<TsType>> = vec![Box::new(ts_quote!("Mixin" as TsType))];
    let ty: TsType = ts_quote!(
        "@{A} & @{B}" as TsType,
        A: TsType = base,
        B: Vec<Box<TsType>> = mixins
    );
    assert_eq!(to_code(&ty), "Base & Mixin");
}

#[test]
fn test_type_lit_vec_splice() {
    let rest: Vec<TsTypeElement> = vec![ts_quote!("age?: number" as TsTypeElement)];
    let ty: TsType = ts_quote!(
        "{ id: string; @{Rest}; }" as TsType,
        Rest: Vec<TsTypeElement> = rest
    );
    assert_eq!(
        to_code(&ty),
        indoc! {"{
            id: string;
            age?: number;
        }"},
    );
}

#[test]
fn test_single_ts_type_element_substitution() {
    let member: TsTypeElement = ts_quote!("age: number" as TsTypeElement);
    let ty: TsType = ts_quote!(
        "{ id: string; @{m}; }" as TsType,
        m: TsTypeElement = member
    );
    assert_eq!(
        to_code(&ty),
        indoc! {"{
            id: string;
            age: number;
        }"},
    );
}

#[test]
fn test_union_vec_lit_str_splice() {
    let variants: Vec<&str> = vec!["active", "inactive"];
    let ty: TsType = ts_quote!(
        "string | @{Variants}" as TsType,
        Variants: Vec<LitStr> = variants
    );
    assert_eq!(to_code(&ty), r#"string | "active" | "inactive""#);
}

// MARK: Type elements

#[test]
fn test_type_element_property() {
    let elem: TsTypeElement = ts_quote!("name: string;" as TsTypeElement);
    assert_eq!(to_code(&elem), "name: string;");
}

#[test]
fn test_type_element_method() {
    let elem: TsTypeElement = ts_quote!("get(id: string): Pet;" as TsTypeElement);
    assert_eq!(to_code(&elem), "get(id: string): Pet;");
}
