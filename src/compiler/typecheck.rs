use std::{
    collections::HashMap,
    fmt::{write, Display},
};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{
    frontend::{
        ast::{BinOpCode, Expression, Function, Item, Parameter, Statement, UnaryOpCode},
        tokenizer::TokenLiteral,
    },
    runtime::{inbuilt::generate_inbuilt_function_hashmap, value::StaticValue},
};

type VariableDefinition = (Type, StaticValue);

pub const INTEGER_TYPE: Type = Type::Primitive(Primitives::Integer);
pub const FLOAT_TYPE: Type = Type::Primitive(Primitives::Float);
pub const CHAR_TYPE: Type = Type::Primitive(Primitives::Char);
pub const BOOL_TYPE: Type = Type::Primitive(Primitives::Bool);
pub const STRING_TYPE: Type = Type::Primitive(Primitives::String);
pub struct TypeChecker<'a> {
    ast: &'a Vec<Item>,
    pub messages: Vec<TypeCheckerMessage>,
    abra_types: HashMap<String, AbraTypeDefinition>,
    global_functions: HashMap<String, FunctionSignature>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub enum Type {
    Null,
    Primitive(Primitives),
    Composite(Box<Composite>),
    Algebraic(Box<Algebraic>),
    Abra(String), // String is the AbraType name
}

impl std::ops::BitOr<Type> for Type {
    type Output = Type;

    fn bitor(self, rhs: Type) -> Self::Output {
        Type::or(self, rhs)
    }
}

impl Type {
    pub fn is_subtype_of(&self, other: &Type) -> bool {
        // Reflexivity: T <: T. Also an optimization.
        if self == other {
            return true;
        }

        // Rule: Null <: T for any T.
        if let Type::Null = self {
            return true;
        }

        if let Type::Null = other {
            return true;
        }

        // Rule: S <: (T1 | T2) if S <: T1 or S <: T2.
        // This applies if 'other' is an Algebraic type.
        if let Type::Algebraic(other_c) = other {
            if let Algebraic::Or(o1, o2) = &**other_c {
                return self.is_subtype_of(o1) || self.is_subtype_of(o2);
            }
        }

        // Rule: (S1 | S2) <: T if S1 <: T and S2 <: T.
        // This applies if 'self' is an Or type and 'other' is not an Or type (that case handled above).
        if let Type::Algebraic(self_c) = self {
            if let Algebraic::Or(s1, s2) = &**self_c {
                return s1.is_subtype_of(other) && s2.is_subtype_of(other);
            }
        }

        // At this point, neither 'self' nor 'other' is an 'Or' type at their top level,
        // or such cases have been resolved. We compare base types or non-Or composites.
        match (self, other) {
            (Type::Primitive(p1), Type::Primitive(p2)) => p1 == p2,
            (Type::Abra(a1), Type::Abra(a2)) => a1 == a2,
            (Type::Composite(sc), Type::Composite(oc)) => {
                // Here, sc and oc are guaranteed not to be Or.
                match (&**sc, &**oc) {
                    (Composite::Array(st), Composite::Array(ot)) => {
                        st.is_subtype_of(ot) // Covariant arrays
                    }
                    (Composite::Map(sk, sv), Composite::Map(ok, ov)) => {
                        // Keys: invariant (s_k <: o_k AND o_k <: s_k)
                        // Values: covariant (s_v <: o_v)
                        (sk.is_subtype_of(ok) && ok.is_subtype_of(sk)) && sv.is_subtype_of(ov)
                    }
                    (Composite::HeapValue(st), Composite::HeapValue(ot)) => {
                        st.is_subtype_of(ot) // Covariant heap values
                    }
                    _ => false, // Different kinds of non-Or composites (e.g., Array vs Map)
                }
            }
            // Any other combination (e.g., Primitive vs. Abra, Primitive vs. non-Or Composite)
            // where subtyping is not explicitly defined is false.
            _ => false,
        }
    }

    pub fn array(t: Type) -> Type {
        Type::Composite(Box::new(Composite::Array(t)))
    }

    pub fn map(k: Type, v: Type) -> Type {
        Type::Composite(Box::new(Composite::Map(k, v)))
    }

    pub fn heap(t: Type) -> Type {
        Type::Composite(Box::new(Composite::HeapValue(t)))
    }

    pub fn or(t1: Type, t2: Type) -> Type {
        Type::Algebraic(Box::new(Algebraic::Or(t1, t2)))
    }

    pub fn abra<S: Into<String>>(name: S) -> Type {
        Type::Abra(name.into())
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Primitive(p) => write!(f, "{}", p),
            Type::Composite(c) => write!(f, "{}", c),
            Type::Abra(a) => write!(f, "{}", a),
            Type::Null => write!(f, "null"),
            Type::Algebraic(algebraic) => write!(f, "({})", algebraic),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub enum Algebraic {
    Or(Type, Type),
}

impl Display for Algebraic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Algebraic::Or(t1, t2) => write!(f, "{} | {}", t1, t2),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub enum Composite {
    Array(Type),
    Map(Type, Type),
    HeapValue(Type),
}

impl Display for Composite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Composite::Array(t) => write!(f, "[{}]", t),
            Composite::Map(k, v) => write!(f, "<{} -> {}>", k, v),
            Composite::HeapValue(t) => write!(f, "Box<{}>", t),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub enum Primitives {
    Integer,
    Float,
    Char,
    Bool,
    String,
}

impl Display for Primitives {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Primitives::Integer => write!(f, "integer"),
            Primitives::Float => write!(f, "float"),
            Primitives::Char => write!(f, "char"),
            Primitives::Bool => write!(f, "bool"),
            Primitives::String => write!(f, "string"),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSignature {
    name: String,
    parameters: Vec<Type>,
    return_type: Type,
}

impl FunctionSignature {
    pub fn new(name: String, parameters: Vec<Type>, return_type: Type) -> Self {
        Self {
            name,
            parameters,
            return_type,
        }
    }
}
impl Display for FunctionSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(", self.name)?;
        for (i, param) in self.parameters.iter().enumerate() {
            write!(f, "{}", param)?;
            if i < self.parameters.len() - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, ") -> {}", self.return_type)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbraTypeDefinition {
    pub name: String,
    pub variables: HashMap<String, VariableDefinition>,
    pub functions: HashMap<String, FunctionSignature>,
}

impl AbraTypeDefinition {
    pub fn new(
        name: String,
        variables: HashMap<String, VariableDefinition>,
        functions: HashMap<String, FunctionSignature>,
    ) -> Self {
        Self {
            name,
            variables,
            functions,
        }
    }
}

impl Display for AbraTypeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "class {} {{", self.name)?;
        for (name, (ty, init)) in &self.variables {
            writeln!(
                // Apply the fix here
                f,
                "  let {}: {} = {}",
                name,
                ty,
                init
            )?;
        }
        for (_, func_sig) in &self.functions {
            writeln!(f, "  func {};", func_sig)?;
        }
        write!(f, "}}")
    }
}

pub enum TypeCheckerMessage {
    Error(anyhow::Error),
    Warning(anyhow::Error),
    Info(anyhow::Error),
}

impl Display for TypeCheckerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeCheckerMessage::Error(e) => write!(f, "Error: {}", e),
            TypeCheckerMessage::Warning(w) => write!(f, "Warning: {}", w),
            TypeCheckerMessage::Info(i) => write!(f, "Info: {}", i),
        }
    }
}

impl<'a> TypeChecker<'a> {
    pub fn new(ast: &'a Vec<Item>) -> Self {
        Self {
            ast: ast,
            messages: Vec::new(),
            abra_types: HashMap::new(),
            global_functions: generate_inbuilt_function_hashmap()
                .into_iter()
                .map(|(k, v)| (k, v.0))
                .collect(),
        }
    }

    pub fn export(
        &self,
    ) -> (
        HashMap<String, AbraTypeDefinition>,
        HashMap<String, FunctionSignature>,
    ) {
        (self.abra_types.clone(), self.global_functions.clone())
    }

    pub fn check(&mut self) {
        //Two pass type-checking system, we don't do it top-to-bottom style like C we are civilized here.
        // First pass: Collect definitions of classes and global functions
        for item in self.ast.iter() {
            match item {
                Item::Class(class) => {
                    let mut ty = AbraTypeDefinition {
                        name: class.name.clone(),
                        variables: HashMap::new(),
                        functions: HashMap::new(),
                    };

                    for var in class.variables.iter() {
                        ty.variables
                            .insert(var.0.clone(), (var.1.clone(), var.2.clone()));
                    }

                    for func in class.functions.iter() {
                        let func_sig = FunctionSignature::new(
                            func.name.clone(),
                            func.params
                                .iter()
                                .map(|p: &Parameter| p.ty.clone())
                                .collect(),
                            func.return_type.clone(),
                        );
                        if ty.functions.insert(func.name.clone(), func_sig).is_some() {
                            self.messages
                                .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                    "Duplicate method definition: '{}' in class '{}'",
                                    func.name,
                                    class.name
                                )));
                        }
                    }
                    if self.abra_types.insert(class.name.clone(), ty).is_some() {
                        self.messages
                            .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Duplicate class definition: {}",
                                class.name
                            )));
                    }
                }
                Item::Function(func) => {
                    let func_sig = FunctionSignature::new(
                        func.name.clone(),
                        func.params
                            .iter()
                            .map(|p: &Parameter| p.ty.clone())
                            .collect(),
                        func.return_type.clone(),
                    );
                    if self
                        .global_functions
                        .insert(func.name.clone(), func_sig)
                        .is_some()
                    {
                        self.messages
                            .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Duplicate global function definition: {}",
                                func.name
                            )));
                    }
                }
            }
        }

        // Second pass: Check function bodies and class method bodies
        for item in self.ast.iter().cloned() {
            match item {
                Item::Class(class) => {
                    // Get the class definition collected in the first pass
                    if let Some(class_def) = self.abra_types.get(&class.name).cloned() {
                        for func in &class.functions {
                            // Initialize scope with 'this'/'self' and class members
                            let mut current_scope_vars = class_def.variables.clone();
                            // Add function parameters to the scope
                            for param in &func.params {
                                if current_scope_vars
                                    .insert(
                                        param.name.clone(),
                                        (param.ty.clone(), StaticValue::Null),
                                    )
                                    .is_some()
                                {
                                    self.messages
                                        .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                        "Parameter '{}' in method '{}::{}' shadows a class member.",
                                        param.name,
                                        class.name,
                                        func.name
                                    )));
                                }
                            }
                            self.check_statement_block(
                                &func.body,
                                &mut current_scope_vars,
                                Some(&func.return_type),
                            );
                        }
                    }
                }
                Item::Function(func) => {
                    let mut current_scope_vars: HashMap<String, VariableDefinition> =
                        HashMap::new();
                    // Add function parameters to the scope
                    for param in &func.params {
                        current_scope_vars
                            .insert(param.name.clone(), (param.ty.clone(), StaticValue::Null));
                    }
                    self.check_statement_block(
                        &func.body,
                        &mut current_scope_vars,
                        Some(&func.return_type),
                    );
                }
            }
        }
    }

    fn check_statement_block(
        &mut self,
        stmts: &Vec<Statement>,
        scope_vars: &mut HashMap<String, VariableDefinition>,
        expected_return_type: Option<&Type>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::Declare(name, declared_type, expr) => {
                    let (expr_type, expr_messages) = self.type_eval_expression(expr, scope_vars);
                    self.messages.extend(expr_messages);
                    if !expr_type.is_subtype_of(declared_type) {
                        self.messages
                            .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Type mismatch in declaration of '{}'. Expected '{}', found '{}'",
                                name,
                                declared_type,
                                expr_type
                            )));
                    }
                    if scope_vars
                        .insert(name.clone(), (declared_type.clone(), StaticValue::Null))
                        .is_some()
                    {
                        self.messages
                            .push(TypeCheckerMessage::Warning(anyhow::anyhow!(
                                "Variable '{}' shadows a variable in an outer scope.",
                                name
                            )));
                    }
                }
                Statement::Set(name, expr) => {
                    if !scope_vars.contains_key(name) {
                        self.messages
                            .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Variable '{}' not found for assignment.",
                                name
                            )));
                        continue;
                    }
                    let (expected_var_type, _) = scope_vars.get(name).unwrap();
                    let (expr_type, expr_messages) = self.type_eval_expression(expr, scope_vars);
                    self.messages.extend(expr_messages);
                    if !expr_type.is_subtype_of(expected_var_type) {
                        self.messages
                            .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Type mismatch in assignment to '{}'. Expected '{}', found '{}'",
                                name,
                                expected_var_type,
                                expr_type
                            )));
                    }
                }
                Statement::Expression(expr) => {
                    let (_, expr_messages) = self.type_eval_expression(expr, scope_vars);
                    self.messages.extend(expr_messages);
                    // Result of expression statement is usually ignored, but errors are collected.
                }
                Statement::Print(expr) => {
                    let (_, expr_messages) = self.type_eval_expression(expr, scope_vars); // Evaluate for side-effects/errors
                    self.messages.extend(expr_messages);
                    // Print can usually take any type, specific checks could be added if needed.
                }
                Statement::Return(opt_expr) => {
                    let return_expr_type = match opt_expr {
                        Some(expr) => {
                            let (t, expr_messages) = self.type_eval_expression(expr, scope_vars);
                            self.messages.extend(expr_messages);
                            t
                        }
                        None => Type::Null, // Or a specific "Void" type if your language has it
                    };
                    if let Some(expected_ret_ty) = expected_return_type {
                        if !return_expr_type.is_subtype_of(expected_ret_ty) {
                            self.messages
                                .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                    "Return type mismatch. Expected '{}', found '{}'",
                                    expected_ret_ty,
                                    return_expr_type
                                )));
                        }
                    } else {
                        self.messages
                            .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Return statement outside of a function."
                            )));
                    }
                }
                Statement::If(cond_expr, then_block, else_opt_block) => {
                    let (cond_type, cond_messages) =
                        self.type_eval_expression(cond_expr, scope_vars);
                    self.messages.extend(cond_messages);
                    if !cond_type.is_subtype_of(&BOOL_TYPE) {
                        self.messages
                            .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "If condition must be a boolean, found '{}'",
                                cond_type
                            )));
                    }
                    let mut then_scope = scope_vars.clone(); // Create a new scope for the 'then' block
                    self.check_statement_block(then_block, &mut then_scope, expected_return_type);
                    if let Some(else_block) = else_opt_block {
                        let mut else_scope = scope_vars.clone(); // Create a new scope for the 'else' block
                        self.check_statement_block(
                            else_block,
                            &mut else_scope,
                            expected_return_type,
                        );
                    }
                }
                Statement::For(init_stmt, cond_expr, incr_stmt, opt_body) => {
                    let mut for_scope = scope_vars.clone(); // New scope for the loop
                    if let init = init_stmt.as_ref() {
                        // Assuming For's init is Option<Box<Statement>>
                        self.check_statement_block(
                            &vec![init.clone()],
                            &mut for_scope,
                            expected_return_type,
                        ); // Check init in the new scope
                    }

                    let (cond_type, cond_messages) =
                        self.type_eval_expression(cond_expr, &for_scope); // Condition uses the new scope
                    self.messages.extend(cond_messages);
                    if !cond_type.is_subtype_of(&BOOL_TYPE) {
                        self.messages
                            .push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "For loop condition must be a boolean, found '{}'",
                                cond_type
                            )));
                    }

                    if let Some(body_stmts) = opt_body {
                        let mut body_scope = for_scope.clone(); // Body also gets its own sub-scope from the for_scope
                        self.check_statement_block(
                            body_stmts,
                            &mut body_scope,
                            expected_return_type,
                        );
                    }

                    if let incr = incr_stmt.as_ref() {
                        // Assuming For's incr is Option<Box<Statement>>
                        self.check_statement_block(
                            &vec![incr.clone()],
                            &mut for_scope,
                            expected_return_type,
                        ); // Increment uses the for_scope
                    }
                }
                Statement::Null => { /* No operation, no type checking needed */ }
            }
        }
    }

    fn type_eval_expression(
        &self,
        e: &Expression,
        variables: &HashMap<String, VariableDefinition>,
    ) -> (Type, Vec<TypeCheckerMessage>) {
        match e {
            Expression::Literal(v) => match v {
                TokenLiteral::Identifier(i) => {
                    if let Some((var_type, _)) = variables.get(i) {
                        (var_type.clone(), Vec::new())
                    } else {
                        (
                            Type::Null,
                            vec![TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Variable {} not found",
                                i
                            ))],
                        )
                    }
                }
                TokenLiteral::Value(static_value) => {
                    let ty = match static_value {
                        StaticValue::Null => Type::Null,
                        StaticValue::Integer(_) => Type::Primitive(Primitives::Integer),
                        StaticValue::Float(_) => Type::Primitive(Primitives::Float),
                        StaticValue::Char(_) => Type::Primitive(Primitives::Char),
                        StaticValue::Bool(_) => Type::Primitive(Primitives::Bool),
                        StaticValue::String(_) => Type::Primitive(Primitives::String),
                    };
                    (ty, Vec::new())
                }
            },
            Expression::Unary(op, expr_box) => {
                let (operand_type_val, mut messages) =
                    self.type_eval_expression(expr_box, variables);

                let result_type = match op {
                    UnaryOpCode::NEG => {
                        if operand_type_val.is_subtype_of(&INTEGER_TYPE) {
                            INTEGER_TYPE
                        } else if operand_type_val.is_subtype_of(&FLOAT_TYPE) {
                            FLOAT_TYPE
                        } else {
                            messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Unary '-' operator cannot be applied to type '{}'",
                                operand_type_val
                            )));
                            Type::Null // Error type
                        }
                    }
                    UnaryOpCode::NOT => {
                        if operand_type_val.is_subtype_of(&BOOL_TYPE) {
                            BOOL_TYPE
                        } else {
                            messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Unary '!' operator cannot be applied to type '{}'",
                                operand_type_val
                            )));
                            Type::Null // Error type
                        }
                    }
                };
                (result_type, vec![])
            }

            Expression::Binary(op, lhs_box, rhs_box) => {
                let (lhs_type_val, mut messages) = self.type_eval_expression(lhs_box, variables);
                let (rhs_type_val, rhs_messages) = self.type_eval_expression(rhs_box, variables);
                messages.extend(rhs_messages);

                // For binary operators, the logic often relies on specific operand types rather than general subtyping for the operation itself.
                // The main change here is for equality operators.
                let result_type = match op {
                    BinOpCode::ADD | BinOpCode::SUB | BinOpCode::MULT | BinOpCode::DIV => {
                        match (&lhs_type_val, &rhs_type_val) {
                            (
                                Type::Primitive(Primitives::Integer),
                                Type::Primitive(Primitives::Integer),
                            ) => Type::Primitive(Primitives::Integer),
                            (
                                Type::Primitive(Primitives::Float),
                                Type::Primitive(Primitives::Float),
                            ) => Type::Primitive(Primitives::Float),
                            (
                                Type::Primitive(Primitives::Integer),
                                Type::Primitive(Primitives::Float),
                            )
                            | (
                                Type::Primitive(Primitives::Float),
                                Type::Primitive(Primitives::Integer),
                            ) => Type::Primitive(Primitives::Float),
                            (
                                Type::Primitive(Primitives::String),
                                Type::Primitive(Primitives::String),
                            ) if *op == BinOpCode::ADD => Type::Primitive(Primitives::String),
                            _ => {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                    "Binary operator '{}' cannot be applied to types '{}' and '{}'",
                                    op,
                                    lhs_type_val,
                                    rhs_type_val
                                )));
                                Type::Null
                            }
                        }
                    }
                    BinOpCode::MOD => match (&lhs_type_val, &rhs_type_val) {
                        (
                            Type::Primitive(Primitives::Integer),
                            Type::Primitive(Primitives::Integer),
                        ) => Type::Primitive(Primitives::Integer),
                        _ => {
                            messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Binary operator '%' cannot be applied to types '{}' and '{}'",
                                lhs_type_val,
                                rhs_type_val
                            )));
                            Type::Null
                        }
                    },
                    BinOpCode::AND | BinOpCode::OR | BinOpCode::XOR => {
                        match (&lhs_type_val, &rhs_type_val) {
                            (
                                Type::Primitive(Primitives::Bool),
                                Type::Primitive(Primitives::Bool),
                            ) => Type::Primitive(Primitives::Bool),
                            _ => {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!("Logical operator '{}' cannot be applied to types '{}' and '{}'", op, lhs_type_val, rhs_type_val)));
                                Type::Null
                            }
                        }
                    }
                    BinOpCode::LT | BinOpCode::LE | BinOpCode::GT | BinOpCode::GE => {
                        match (&lhs_type_val, &rhs_type_val) {
                            (
                                Type::Primitive(Primitives::Integer),
                                Type::Primitive(Primitives::Integer),
                            )
                            | (
                                Type::Primitive(Primitives::Float),
                                Type::Primitive(Primitives::Float),
                            )
                            | (
                                Type::Primitive(Primitives::Integer),
                                Type::Primitive(Primitives::Float),
                            )
                            | (
                                Type::Primitive(Primitives::Float),
                                Type::Primitive(Primitives::Integer),
                            )
                            | (
                                Type::Primitive(Primitives::Char),
                                Type::Primitive(Primitives::Char),
                            ) => Type::Primitive(Primitives::Bool),
                            _ => {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!("Comparison operator '{}' cannot be applied to types '{}' and '{}'", op, lhs_type_val, rhs_type_val)));
                                Type::Null
                            }
                        }
                    }
                    BinOpCode::EQ | BinOpCode::NE => {
                        match (&lhs_type_val, &rhs_type_val) {
                            (Type::Primitive(p1), Type::Primitive(p2)) if p1 == p2 => {
                                Type::Primitive(Primitives::Bool)
                            }
                            (
                                Type::Primitive(Primitives::Integer),
                                Type::Primitive(Primitives::Float),
                            )
                            | (
                                Type::Primitive(Primitives::Float),
                                Type::Primitive(Primitives::Integer),
                            ) => Type::Primitive(Primitives::Bool),
                            (Type::Abra(a1), Type::Abra(a2)) if a1 == a2 => {
                                Type::Primitive(Primitives::Bool)
                            }
                            (Type::Null, Type::Null) => Type::Primitive(Primitives::Bool),
                            (_, Type::Null) | (Type::Null, _) => Type::Primitive(Primitives::Bool),
                            // Use subtyping for general comparability
                            _ if lhs_type_val.is_subtype_of(&rhs_type_val)
                                || rhs_type_val.is_subtype_of(&lhs_type_val) =>
                            {
                                Type::Primitive(Primitives::Bool)
                            }
                            _ => {
                                // Consider if this should be a warning or if some comparisons are always false but not errors
                                messages.push(TypeCheckerMessage::Warning(anyhow::anyhow!("Equality operator '{}' may not behave as expected for types '{}' and '{}'", op, lhs_type_val, rhs_type_val)));
                                Type::Null
                            }
                        }
                    }
                };
                (result_type, messages)
            }
            Expression::Grouping(expr_box) => self.type_eval_expression(expr_box, variables),
            Expression::Call(func_name, arg_exprs_vec) => {
                let mut messages: Vec<TypeCheckerMessage> = Vec::new();
                let mut return_ty = Type::Null;

                if let Some(func_sig) = self.global_functions.get(func_name) {
                    return_ty = func_sig.return_type.clone();
                    if arg_exprs_vec.len() != func_sig.parameters.len() {
                        messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                            "Function '{}' expected {} arguments, but got {}",
                            func_name,
                            func_sig.parameters.len(),
                            arg_exprs_vec.len()
                        )));
                    } else {
                        for (i, arg_expr) in arg_exprs_vec.iter().enumerate() {
                            let (arg_type_val, arg_messages) =
                                self.type_eval_expression(arg_expr, variables);
                            messages.extend(arg_messages);
                            if !arg_type_val.is_subtype_of(&func_sig.parameters[i]) {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!("Argument {} for function '{}': expected type '{}', but got '{}'", i + 1, func_name, func_sig.parameters[i], arg_type_val)));
                            }
                        }
                    }
                } else {
                    messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                        "Global function '{}' not found",
                        func_name
                    )));
                }
                (return_ty, messages)
            }
            Expression::Get(member_name, base_expr) => {
                let (base_type_val, mut messages) = self.type_eval_expression(base_expr, variables);

                let result_type = match base_type_val {
                    Type::Abra(class_name_str) => {
                        if let Some(class_def) = self.abra_types.get(&class_name_str) {
                            if let Some((var_type, _)) = class_def.variables.get(member_name) {
                                var_type.clone()
                            } else if class_def.functions.contains_key(member_name) {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!("Accessing method '{}' on class '{}' as a value is not directly supported. Call it with ().", member_name, class_name_str)));
                                Type::Null // Or a specific function/method type if the language supports it
                            } else {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                    "Member '{}' not found in class '{}'",
                                    member_name,
                                    class_name_str
                                )));
                                Type::Null
                            }
                        } else {
                            messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Class definition '{}' not found for access",
                                class_name_str
                            )));
                            Type::Null
                        }
                    }
                    Type::Composite(ref composite_box) => match **composite_box {
                        Composite::Array(_) if member_name == "length" => {
                            Type::Primitive(Primitives::Integer)
                        }
                        Composite::Map(_, _) if member_name == "size" => {
                            Type::Primitive(Primitives::Integer)
                        } // Example property
                        _ => {
                            messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Member access '{}' not supported on type '{}'",
                                member_name,
                                base_type_val
                            )));
                            Type::Null
                        }
                    },
                    Type::Primitive(Primitives::String) if member_name == "length" => {
                        Type::Primitive(Primitives::Integer)
                    }
                    _ => {
                        messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                            "Cannot access member '{}' on type '{}'",
                            member_name,
                            base_type_val
                        )));
                        Type::Null
                    }
                };
                (result_type, messages)
            }
            Expression::Instance(ty, arg_exprs_vec) => {
                let mut result_type = ty.clone();
                let mut messages: Vec<TypeCheckerMessage> = Vec::new();

                match ty.clone() {
                    Type::Abra(class_name) => {
                        if let Some(class_def) = self.abra_types.get(&class_name) {
                            let constructor_sig_opt = class_def.functions.get("init"); // Assuming constructor is 'init'
                            if let Some(constructor_sig) = constructor_sig_opt {
                                if arg_exprs_vec.len() != constructor_sig.parameters.len() {
                                    messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                        "Constructor for '{}' expected {} arguments, but got {}",
                                        class_name,
                                        constructor_sig.parameters.len(),
                                        arg_exprs_vec.len()
                                    )));
                                } else {
                                    for (i, arg_expr) in arg_exprs_vec.iter().enumerate() {
                                        let (arg_type_val, arg_eval_messages) =
                                            self.type_eval_expression(arg_expr, variables);
                                        messages.extend(arg_eval_messages);
                                        if !arg_type_val
                                            .is_subtype_of(&constructor_sig.parameters[i])
                                        {
                                            messages.push(TypeCheckerMessage::Error(anyhow::anyhow!("Argument {} for '{}' constructor: expected type '{}', but got '{}'", i + 1, class_name, constructor_sig.parameters[i], arg_type_val)));
                                        }
                                    }
                                }
                            } else if !arg_exprs_vec.is_empty() {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!("Class '{}' does not have an 'init' constructor, but arguments were provided.", class_name)));
                            }
                        } else {
                            messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                "Cannot instantiate unknown class '{}'",
                                class_name
                            )));
                            result_type = Type::Null;
                        }
                    }
                    Type::Composite(composite_box) => match *composite_box {
                        Composite::Array(ref element_type) => {
                            for arg_expr in arg_exprs_vec {
                                let (arg_type_val, arg_eval_messages) =
                                    self.type_eval_expression(arg_expr, variables);
                                messages.extend(arg_eval_messages);
                                if !arg_type_val.is_subtype_of(element_type) {
                                    messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                        "Array element expected type '{}', but got '{}'",
                                        element_type,
                                        arg_type_val
                                    )));
                                }
                            }
                        }
                        Composite::Map(ref key_type, ref value_type) => {
                            if arg_exprs_vec.len() % 2 != 0 {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!("Map instantiation requires an even number of arguments (key-value pairs), got {}", arg_exprs_vec.len())));
                            } else {
                                for chunk in arg_exprs_vec.chunks_exact(2) {
                                    let (k_actual_type_val, k_eval_messages) =
                                        self.type_eval_expression(&chunk[0], variables);
                                    messages.extend(k_eval_messages);
                                    let (v_actual_type_val, v_eval_messages) =
                                        self.type_eval_expression(&chunk[1], variables);
                                    messages.extend(v_eval_messages);

                                    if !k_actual_type_val.is_subtype_of(key_type) {
                                        messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                            "Map key expected type '{}', but got '{}'",
                                            key_type,
                                            k_actual_type_val
                                        )));
                                    }
                                    if !v_actual_type_val.is_subtype_of(value_type) {
                                        messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                            "Map value expected type '{}', but got '{}'",
                                            value_type,
                                            v_actual_type_val
                                        )));
                                    }
                                }
                            }
                        }
                        Composite::HeapValue(ref inner_type) => {
                            if arg_exprs_vec.len() != 1 {
                                messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                    "Box (HeapValue) instantiation expects 1 argument, got {}",
                                    arg_exprs_vec.len()
                                )));
                            } else {
                                let (arg_type_val, arg_eval_messages) =
                                    self.type_eval_expression(&arg_exprs_vec[0], variables);
                                messages.extend(arg_eval_messages);
                                if !arg_type_val.is_subtype_of(inner_type) {
                                    messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                                        "Box (HeapValue) expected inner type '{}', but got '{}'",
                                        inner_type,
                                        arg_type_val
                                    )));
                                }
                            }
                        }
                    },
                    Type::Algebraic(_) => {
                        messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                            "Cannot instantiate algebraic type '{}' using 'new'",
                            ty
                        )));
                        result_type = Type::Null;
                    }
                    Type::Primitive(_) | Type::Null => {
                        messages.push(TypeCheckerMessage::Error(anyhow::anyhow!(
                            "Cannot instantiate primitive type '{}' or Null using 'new'",
                            ty
                        )));
                        result_type = Type::Null;
                    }
                }
                (result_type, messages)
            }
        }
    }
}
