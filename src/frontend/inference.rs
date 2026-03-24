use crate::{compiler::typecheck::Type, frontend::ast::{Expression, Statement}, runtime::value::Value};
#[derive(Debug,PartialEq, PartialOrd)]
pub enum InferenceResult{
    Unknown,
    Dynamic,
    TypeOfVar(String),
    TypeOfFuncRet(String),
    Exact(Type)
}

fn and(a: InferenceResult, b: InferenceResult) -> InferenceResult {
    
}

fn or(a: InferenceResult, b: InferenceResult) -> InferenceResult {

}

pub fn infer_type(expr: &Expression) -> InferenceResult {
    match expr {
        Expression::Literal(token_literal) => match token_literal {
            crate::frontend::tokenizer::TokenLiteral::Identifier(ident) => InferenceResult::TypeOfVar(ident.clone()),
            crate::frontend::tokenizer::TokenLiteral::Value(static_value) => InferenceResult::Exact(static_value.into::<Value>().into()),
        },
        Expression::Unary(unary_op_code, expression) => infer_type(expression),
        Expression::Binary(bin_op_code, expression, expression1) => {

        },
        Expression::Grouping(expression) => infer_type(&expression),
        Expression::Call(string, expressions) => InferenceResult::TypeOfFuncRet(string.clone()),
        Expression::Get(string, expression) => InferenceResult::TypeOfVar(string.clone()),
        Expression::Instance(ty, expressions) => InferenceResult::Exact(ty.clone()),
    }
}
