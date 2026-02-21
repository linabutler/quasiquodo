use quasiquodo::ts_quote;
use swc_ecma_ast::*;
use swc_ecma_codegen::to_code;

// MARK: `ClassMember` output kind

#[test]
fn test_class_member_property() {
    let member: ClassMember = ts_quote!("name: string;" as ClassMember);
    assert_eq!(to_code(&member), "name: string;");
}

#[test]
fn test_class_member_method() {
    let member: ClassMember = ts_quote!("async get(): Promise<void> {}" as ClassMember);
    assert_eq!(to_code(&member), "async get(): Promise<void> {}");
}

// MARK: Method parameter splicing

#[test]
fn test_method_params_vec_splice() {
    let params: Vec<Param> = vec![ts_quote!("id: string" as Param)];
    let member: ClassMember = ts_quote!(
        "async get(@{Params}): Promise<void> {}" as ClassMember,
        Params: Vec<Param> = params,
    );
    assert_eq!(to_code(&member), "async get(id: string): Promise<void> {}");
}

// MARK: Constructor parameter splicing

#[test]
fn test_constructor_params_vec_splice() {
    let params: Vec<ParamOrTsParamProp> = vec![ts_quote!("x: string" as ParamOrTsParamProp)];
    let member: ClassMember = ts_quote!(
        "constructor(@{Params}) {}" as ClassMember,
        Params: Vec<ParamOrTsParamProp> = params,
    );
    assert_eq!(to_code(&member), "constructor(x: string){}");
}

#[test]
fn test_constructor_fixed_plus_vec_splice() {
    let rest: Vec<ParamOrTsParamProp> = vec![ts_quote!("y: number" as ParamOrTsParamProp)];
    let member: ClassMember = ts_quote!(
        "constructor(x: string, @{Rest}) {}" as ClassMember,
        Rest: Vec<ParamOrTsParamProp> = rest,
    );
    assert_eq!(to_code(&member), "constructor(x: string, y: number){}");
}

#[test]
fn test_single_param_or_ts_param_prop_substitution() {
    let my_param: ParamOrTsParamProp = ts_quote!("x: string" as ParamOrTsParamProp);
    let member: ClassMember = ts_quote!(
        "constructor(@{p}) {}" as ClassMember,
        p: ParamOrTsParamProp = my_param,
    );
    assert_eq!(to_code(&member), "constructor(x: string){}");
}

// MARK: `ParamOrTsParamProp` output kind

#[test]
fn test_param_or_ts_param_prop_output() {
    let p: ParamOrTsParamProp = ts_quote!("x: string" as ParamOrTsParamProp);
    assert_eq!(to_code(&p), "x: string");
}

#[test]
fn test_param_or_ts_param_prop_with_variable() {
    let my_name: Ident = ts_quote!("x" as Ident);
    let my_ty: TsType = ts_quote!("number" as TsType);
    let p: ParamOrTsParamProp = ts_quote!(
        "@{name}: @{ty}" as ParamOrTsParamProp,
        name: Ident = my_name,
        ty: TsType = my_ty,
    );
    assert_eq!(to_code(&p), "x: number");
}
