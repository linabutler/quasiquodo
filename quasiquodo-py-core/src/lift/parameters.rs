use ruff_python_ast::*;
use syn::parse_quote;

use crate::context::Context;

use super::{CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, lift_variants};

impl_lift_for_struct!(Parameter, [range, node_index, name, annotation]);

impl_lift_for_struct!(
    ParameterWithDefault,
    [range, node_index, parameter, default]
);

impl_lift_for_struct!(
    Parameters,
    [
        range,
        node_index,
        posonlyargs,
        args,
        vararg,
        kwonlyargs,
        kwarg
    ]
);

impl_lift_for_struct!(Keyword, [range, node_index, arg, value]);

impl_lift_for_struct!(Arguments, [range, node_index, args, keywords]);

impl_lift_for_struct!(Decorator, [range, node_index, expression]);

impl_lift_for_struct!(Alias, [range, node_index, name, asname]);

impl_lift_for_struct!(WithItem, [range, node_index, context_expr, optional_vars]);

impl_lift_for_struct!(
    Comprehension,
    [range, node_index, target, iter, ifs, is_async]
);

impl_lift_for_struct!(ElifElseClause, [range, node_index, test, body]);

impl_lift_for_struct!(MatchCase, [range, node_index, pattern, guard, body]);

impl_lift_for_newtype_enum!(
    Pattern,
    [
        MatchValue,
        MatchSingleton,
        MatchSequence,
        MatchMapping,
        MatchClass,
        MatchStar,
        MatchAs,
        MatchOr
    ]
);

impl_lift_for_struct!(PatternMatchValue, [node_index, range, value]);

impl_lift_for_struct!(PatternMatchSingleton, [node_index, range, value]);

impl_lift_for_struct!(PatternMatchSequence, [node_index, range, patterns]);

impl_lift_for_struct!(
    PatternMatchMapping,
    [node_index, range, keys, patterns, rest]
);

impl_lift_for_struct!(PatternMatchClass, [node_index, range, cls, arguments]);

impl_lift_for_struct!(PatternMatchStar, [node_index, range, name]);

impl_lift_for_struct!(PatternMatchAs, [node_index, range, pattern, name]);

impl_lift_for_struct!(PatternMatchOr, [node_index, range, patterns]);

impl_lift_for_struct!(PatternArguments, [range, node_index, patterns, keywords]);

impl_lift_for_struct!(PatternKeyword, [range, node_index, attr, pattern]);

impl_lift_for_struct!(TypeParams, [range, node_index, type_params]);

impl_lift_for_newtype_enum!(TypeParam, [TypeVar, TypeVarTuple, ParamSpec]);

impl_lift_for_struct!(TypeParamTypeVar, [node_index, range, name, bound, default]);

impl_lift_for_struct!(TypeParamTypeVarTuple, [node_index, range, name, default]);

impl_lift_for_struct!(TypeParamParamSpec, [node_index, range, name, default]);
