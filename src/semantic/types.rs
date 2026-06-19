// src/semantic/types.rs
use super::SemanticAnalyzer;
use super::suggestions::suggest_similar_field;
use crate::ast::*;

impl SemanticAnalyzer {
    /// Checks if a standard library identifier can be accessed under the current configuration.
    fn check_std_access(&self, name: &str) -> Result<DataType, String> {
        if name == "time" {
            if !self.capabilities.contains(&"time".to_string()) {
                return Err(
                    "Missing capability 'time' required to use 'time' standard library".to_string(),
                );
            }
            if self.in_render_block {
                return Err("Standard library 'time' is not allowed in render blocks. Views must be pure rendering layers.".to_string());
            }
            return Ok(DataType::Custom("StdTime".to_string()));
        }
        if name == "http" {
            if !self.capabilities.contains(&"network.outbound".to_string()) {
                return Err(
                    "Missing capability 'network.outbound' required to use 'http' standard library"
                        .to_string(),
                );
            }
            if self.in_render_block {
                return Err("Standard library 'http' is not allowed in render blocks. Views must be pure rendering layers.".to_string());
            }
            return Ok(DataType::Custom("StdHttp".to_string()));
        }
        if name == "auth" {
            if !self.capabilities.contains(&"auth".to_string()) {
                return Err(
                    "Missing capability 'auth' required to use 'auth' standard library".to_string(),
                );
            }
            if self.in_render_block {
                return Err("Standard library 'auth' is not allowed in render blocks. Views must be pure rendering layers.".to_string());
            }
            return Ok(DataType::Custom("StdAuth".to_string()));
        }

        let err_msg = if let Some(suggested) = self.suggest_similar_variable(name) {
            format!(
                "Undefined variable: '{}'. Did you mean '{}'?",
                name, suggested
            )
        } else {
            format!("Undefined variable: '{}'", name)
        };
        Err(err_msg)
    }

    /// Evaluates and checks the DataType of an AST Expression node, verifying coercions and rules.
    pub fn check_expression_type(&self, expr: &Expression) -> Result<DataType, String> {
        match expr {
            Expression::Number(_) => Ok(DataType::Float),
            Expression::StringLiteral(_) => Ok(DataType::Str),
            Expression::Boolean(_) => Ok(DataType::Bool),
            Expression::Null => Ok(DataType::Str),

            Expression::Identifier(name) => {
                if name == "time" || name == "http" || name == "auth" {
                    self.check_std_access(name)
                } else if name == "env" {
                    Ok(DataType::Custom("StdEnv".to_string()))
                } else {
                    self.resolve_symbol(name)
                        .map(|sym| sym.data_type.clone())
                        .ok_or_else(|| {
                            if let Some(suggested) = self.suggest_similar_variable(name) {
                                format!(
                                    "Undefined variable: '{}'. Did you mean '{}'?",
                                    name, suggested
                                )
                            } else {
                                format!("Undefined variable: '{}'", name)
                            }
                        })
                }
            }

            Expression::Binary { left, op, right } => {
                let left_type = self.check_expression_type(left)?;
                let right_type = self.check_expression_type(right)?;

                match op.as_str() {
                    "+" | "-" | "*" | "/" => {
                        if self.is_numeric(&left_type) && self.is_numeric(&right_type) {
                            if left_type == DataType::Money || right_type == DataType::Money {
                                Ok(DataType::Money)
                            } else if left_type == DataType::Float || right_type == DataType::Float
                            {
                                Ok(DataType::Float)
                            } else {
                                Ok(DataType::Int)
                            }
                        } else if op == "+"
                            && (left_type == DataType::Str || right_type == DataType::Str)
                        {
                            Ok(DataType::Str)
                        } else {
                            Err(format!(
                                "Arithmetic operation '{}' is not valid for types {:?} and {:?}",
                                op, left_type, right_type
                            ))
                        }
                    }
                    "=" => {
                        match &**left {
                            Expression::Identifier(_) | Expression::MemberAccess { .. } => {}
                            _ => {
                                return Err(
                                    "Left side of assignment must be a variable or property"
                                        .to_string(),
                                );
                            }
                        }
                        if left_type == right_type
                            || (self.is_numeric(&left_type) && self.is_numeric(&right_type))
                            || matches!(**right, Expression::Null)
                            || left_type == DataType::Str
                        {
                            Ok(left_type)
                        } else {
                            Err(format!(
                                "Cannot assign type {:?} to variable of type {:?}",
                                right_type, left_type
                            ))
                        }
                    }
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                        if left_type == right_type
                            || (self.is_numeric(&left_type) && self.is_numeric(&right_type))
                            || matches!(**left, Expression::Null)
                            || matches!(**right, Expression::Null)
                        {
                            Ok(DataType::Bool)
                        } else {
                            Err(format!(
                                "Comparison '{}' is not valid between types {:?} and {:?}",
                                op, left_type, right_type
                            ))
                        }
                    }
                    "and" | "or" => {
                        if left_type == DataType::Bool && right_type == DataType::Bool {
                            Ok(DataType::Bool)
                        } else {
                            Err(format!(
                                "Logical operation '{}' is only valid for boolean operands, got {:?} and {:?}",
                                op, left_type, right_type
                            ))
                        }
                    }
                    _ => Err(format!("Unsupported binary operator '{}'", op)),
                }
            }

            Expression::Unary { op, expr } => {
                let expr_type = self.check_expression_type(expr)?;
                match op.as_str() {
                    "not" | "!" => {
                        if expr_type == DataType::Bool {
                            Ok(DataType::Bool)
                        } else {
                            Err(format!(
                                "Unary operation '{}' requires boolean operand, got {:?}",
                                op, expr_type
                            ))
                        }
                    }
                    "-" => {
                        if self.is_numeric(&expr_type) {
                            Ok(expr_type)
                        } else {
                            Err(format!(
                                "Unary negation requires numeric operand, got {:?}",
                                expr_type
                            ))
                        }
                    }
                    _ => Err(format!("Unsupported unary operator '{}'", op)),
                }
            }

            Expression::MemberAccess { object, property } => {
                let object_type = self.check_expression_type(object)?;
                match object_type {
                    DataType::Custom(ref s)
                        if s == "StdTime" || s == "StdHttp" || s == "StdAuth" =>
                    {
                        Ok(DataType::Custom("StdFunc".to_string()))
                    }
                    DataType::Custom(ref s) if s == "RequestMap" => Ok(DataType::Str),
                    DataType::Model(model_name) => {
                        if model_name == self.auth_model && property == "current" {
                            return Ok(DataType::Model(model_name));
                        }
                        if property == "id" {
                            return Ok(DataType::Int);
                        }

                        let model = self.models.get(&model_name).ok_or_else(|| {
                            format!("Model '{}' not found in database schema.", model_name)
                        })?;

                        let field = model.fields.iter().find(|f| f.name == *property)
                            .ok_or_else(|| {
                                if let Some(suggested) = suggest_similar_field(property, model) {
                                    format!("Property '{}' does not exist in model '{}'. Did you mean '{}'?", property, model_name, suggested)
                                } else {
                                    format!("Property '{}' does not exist in model '{}'", property, model_name)
                                }
                            })?;

                        Ok(field.data_type.clone())
                    }
                    DataType::List(_) => {
                        if property == "length" {
                            Ok(DataType::Int)
                        } else {
                            Err(format!(
                                "Cannot read property '{}' on non-model type {:?}",
                                property, object_type
                            ))
                        }
                    }
                    _ => Err(format!(
                        "Cannot read property '{}' on non-model type {:?}",
                        property, object_type
                    )),
                }
            }

            Expression::Call { callee, args } => {
                let callee_type = self.check_expression_type(callee)?;
                match callee_type {
                    DataType::Custom(ref s) if s == "StdEnv" => {
                        if args.is_empty() || args.len() > 2 {
                            return Err(
                                "Function 'env' expects 1 or 2 string arguments".to_string()
                            );
                        }
                        for arg in args {
                            let arg_type = self.check_expression_type(arg)?;
                            if arg_type != DataType::Str {
                                return Err(format!(
                                    "Function 'env' arguments must be string, got {:?}",
                                    arg_type
                                ));
                            }
                        }
                        Ok(DataType::Str)
                    }
                    DataType::Custom(ref s) if s == "StdFunc" => {
                        for arg in args {
                            self.check_expression_type(arg)?;
                        }
                        Ok(DataType::Str)
                    }
                    DataType::Model(model_name) => Ok(DataType::Model(model_name)),
                    _ => {
                        let _arg_types = args
                            .iter()
                            .map(|arg| self.check_expression_type(arg))
                            .collect::<Result<Vec<DataType>, String>>()?;
                        Ok(DataType::Str)
                    }
                }
            }

            Expression::Ternary {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.check_expression_type(cond)?;
                if cond_type != DataType::Bool {
                    return Err(format!(
                        "Ternary condition must be boolean, got {:?}",
                        cond_type
                    ));
                }
                let then_type = self.check_expression_type(then_branch)?;
                let else_type = self.check_expression_type(else_branch)?;
                if then_type == else_type {
                    Ok(then_type)
                } else if self.is_numeric(&then_type) && self.is_numeric(&else_type) {
                    if then_type == DataType::Money || else_type == DataType::Money {
                        Ok(DataType::Money)
                    } else if then_type == DataType::Float || else_type == DataType::Float {
                        Ok(DataType::Float)
                    } else {
                        Ok(DataType::Int)
                    }
                } else if then_type == DataType::Str || else_type == DataType::Str {
                    Ok(DataType::Str)
                } else {
                    Err(format!(
                        "Ternary branches must have matching types, got {:?} and {:?}",
                        then_type, else_type
                    ))
                }
            }
        }
    }

    /// Evaluates if a given DataType is numeric (Int, Float, or Money).
    fn is_numeric(&self, dt: &DataType) -> bool {
        matches!(dt, DataType::Int | DataType::Float | DataType::Money)
    }

    pub(crate) fn types_compatible(&self, expected: &DataType, actual: &DataType) -> bool {
        expected == actual
            || (self.is_numeric(expected) && self.is_numeric(actual))
            || matches!(
                (expected, actual),
                (
                    DataType::Email | DataType::Password | DataType::DateTime,
                    DataType::Str
                )
            )
            || *expected == DataType::Str
    }

    pub(crate) fn field_type_for(model: &ModelDecl, field_name: &str) -> Option<DataType> {
        if field_name.eq_ignore_ascii_case("id") {
            return Some(DataType::Int);
        }
        model
            .fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(field_name))
            .map(|field| field.data_type.clone())
    }

    pub(crate) fn expression_uses_current_user(&self, expr: &Expression) -> bool {
        match expr {
            Expression::MemberAccess { object, property } if property == "current" => {
                matches!(&**object, Expression::Identifier(name) if name == &self.auth_model)
            }
            Expression::MemberAccess { object, .. } => self.expression_uses_current_user(object),
            Expression::Binary { left, right, .. } => {
                self.expression_uses_current_user(left) || self.expression_uses_current_user(right)
            }
            Expression::Unary { expr, .. } => self.expression_uses_current_user(expr),
            Expression::Ternary {
                cond,
                then_branch,
                else_branch,
            } => {
                self.expression_uses_current_user(cond)
                    || self.expression_uses_current_user(then_branch)
                    || self.expression_uses_current_user(else_branch)
            }
            Expression::Call { callee, args } => {
                self.expression_uses_current_user(callee)
                    || args
                        .iter()
                        .any(|arg| self.expression_uses_current_user(arg))
            }
            _ => false,
        }
    }
}
