//! Portable interpreter runtime for Cobra's first execution target.

use cobra_parser_ast::{BinaryOp, Expr, Literal, Program, Stmt, UnaryOp};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[derive(Clone)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(String),
    Function(Rc<FunctionValue>),
    Null,
}

impl Display for Value {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(number) if number.fract() == 0.0 => write!(formatter, "{number:.0}"),
            Self::Number(number) => write!(formatter, "{number}"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::String(value) => write!(formatter, "{value}"),
            Self::Function(function) => write!(formatter, "<fn {}>", function.name),
            Self::Null => write!(formatter, "null"),
        }
    }
}

impl Value {
    fn is_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Null, Self::Null) => true,
            _ => false,
        }
    }
}

pub struct FunctionValue {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    closure: EnvironmentRef,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError {
    pub message: String,
}

impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

type EnvironmentRef = Rc<RefCell<Environment>>;

struct Environment {
    values: HashMap<String, Value>,
    parent: Option<EnvironmentRef>,
}

impl Environment {
    fn root() -> EnvironmentRef {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: None,
        }))
    }

    fn child(parent: &EnvironmentRef) -> EnvironmentRef {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: Some(parent.clone()),
        }))
    }

    fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    fn get(environment: &EnvironmentRef, name: &str) -> Option<Value> {
        let (value, parent) = {
            let current = environment.borrow();
            (current.values.get(name).cloned(), current.parent.clone())
        };
        value.or_else(|| parent.and_then(|parent| Self::get(&parent, name)))
    }
}

pub fn interpret(program: &Program) -> Result<Value, RuntimeError> {
    let environment = Environment::root();
    match execute_statements(&program.statements, &environment)? {
        Flow::Normal(value) | Flow::Return(value) => Ok(value),
    }
}

enum Flow {
    Normal(Value),
    Return(Value),
}

fn execute_statements(
    statements: &[Stmt],
    environment: &EnvironmentRef,
) -> Result<Flow, RuntimeError> {
    let mut last = Value::Null;
    for statement in statements {
        match execute_statement(statement, environment)? {
            Flow::Normal(value) => last = value,
            Flow::Return(value) => return Ok(Flow::Return(value)),
        }
    }
    Ok(Flow::Normal(last))
}

fn execute_statement(statement: &Stmt, environment: &EnvironmentRef) -> Result<Flow, RuntimeError> {
    match statement {
        Stmt::Let {
            name, initializer, ..
        } => {
            let value = evaluate(initializer, environment)?;
            environment.borrow_mut().define(name.clone(), value);
            Ok(Flow::Normal(Value::Null))
        }
        Stmt::Expression { expression, .. } => Ok(Flow::Normal(evaluate(expression, environment)?)),
        Stmt::Block { statements, .. } => {
            let child = Environment::child(environment);
            execute_statements(statements, &child)
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            if is_truthy(&evaluate(condition, environment)?) {
                execute_statements(then_branch, &Environment::child(environment))
            } else if let Some(else_branch) = else_branch {
                execute_statements(else_branch, &Environment::child(environment))
            } else {
                Ok(Flow::Normal(Value::Null))
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            let mut last = Value::Null;
            let mut iterations = 0usize;
            while is_truthy(&evaluate(condition, environment)?) {
                iterations += 1;
                if iterations > 1_000_000 {
                    return Err(RuntimeError {
                        message: "loop exceeded the safety limit of 1,000,000 iterations".into(),
                    });
                }
                // The loop body shares its environment so explicit shadowing
                // can advance a loop variable on the next condition check.
                match execute_statements(body, environment)? {
                    Flow::Normal(value) => last = value,
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                }
            }
            Ok(Flow::Normal(last))
        }
        Stmt::Function {
            name, params, body, ..
        } => {
            let function = FunctionValue {
                name: name.clone(),
                params: params.clone(),
                body: body.clone(),
                closure: environment.clone(),
            };
            environment
                .borrow_mut()
                .define(name.clone(), Value::Function(Rc::new(function)));
            Ok(Flow::Normal(Value::Null))
        }
        Stmt::Return { value, .. } => {
            let value = value
                .as_ref()
                .map(|expression| evaluate(expression, environment))
                .transpose()?
                .unwrap_or(Value::Null);
            Ok(Flow::Return(value))
        }
    }
}

fn evaluate(expression: &Expr, environment: &EnvironmentRef) -> Result<Value, RuntimeError> {
    match expression {
        Expr::Literal(Literal::Number(value)) => Ok(Value::Number(*value)),
        Expr::Literal(Literal::String(value)) => Ok(Value::String(value.clone())),
        Expr::Literal(Literal::Bool(value)) => Ok(Value::Bool(*value)),
        Expr::Literal(Literal::Null) => Ok(Value::Null),
        Expr::Variable(name) => Environment::get(environment, name).ok_or_else(|| RuntimeError {
            message: format!("undefined variable `{name}`"),
        }),
        Expr::Unary { operator, right } => {
            let value = evaluate(right, environment)?;
            match operator {
                UnaryOp::Not => Ok(Value::Bool(!is_truthy(&value))),
                UnaryOp::Negate => match value {
                    Value::Number(number) => Ok(Value::Number(-number)),
                    _ => Err(RuntimeError {
                        message: "unary `-` expects a number".into(),
                    }),
                },
            }
        }
        Expr::Binary {
            left,
            operator,
            right,
        } => {
            let left = evaluate(left, environment)?;
            let right = evaluate(right, environment)?;
            evaluate_binary(left, *operator, right)
        }
        Expr::Call { callee, arguments } => {
            if let Expr::Variable(name) = callee.as_ref() {
                if name == "print" {
                    let values = arguments
                        .iter()
                        .map(|argument| evaluate(argument, environment))
                        .collect::<Result<Vec<_>, _>>()?;
                    println!(
                        "{}",
                        values
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                    return Ok(Value::Null);
                }
                if name == "type_of" {
                    if arguments.len() != 1 {
                        return Err(RuntimeError {
                            message: "`type_of` expects exactly one argument".into(),
                        });
                    }
                    let value = evaluate(&arguments[0], environment)?;
                    let name = match value {
                        Value::Number(_) => "number",
                        Value::Bool(_) => "bool",
                        Value::String(_) => "string",
                        Value::Function(_) => "function",
                        Value::Null => "null",
                    };
                    return Ok(Value::String(name.into()));
                }
            }

            let callee = evaluate(callee, environment)?;
            let Value::Function(function) = callee else {
                return Err(RuntimeError {
                    message: "only functions can be called".into(),
                });
            };
            if arguments.len() != function.params.len() {
                return Err(RuntimeError {
                    message: format!(
                        "function `{}` expects {} arguments, got {}",
                        function.name,
                        function.params.len(),
                        arguments.len()
                    ),
                });
            }
            let call_environment = Environment::child(&function.closure);
            for (parameter, argument) in function.params.iter().zip(arguments) {
                let value = evaluate(argument, environment)?;
                call_environment
                    .borrow_mut()
                    .define(parameter.clone(), value);
            }
            match execute_statements(&function.body, &call_environment)? {
                Flow::Normal(value) | Flow::Return(value) => Ok(value),
            }
        }
    }
}

fn evaluate_binary(left: Value, operator: BinaryOp, right: Value) -> Result<Value, RuntimeError> {
    match operator {
        BinaryOp::Add => match (left, right) {
            (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left + right)),
            (Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
            (Value::String(left), right) => Ok(Value::String(left + &right.to_string())),
            (left, Value::String(right)) => Ok(Value::String(left.to_string() + &right)),
            _ => Err(RuntimeError {
                message: "`+` expects numbers or strings".into(),
            }),
        },
        BinaryOp::Equal => Ok(Value::Bool(left.is_equal(&right))),
        BinaryOp::NotEqual => Ok(Value::Bool(!left.is_equal(&right))),
        operator => {
            let (left, right) = match (left, right) {
                (Value::Number(left), Value::Number(right)) => (left, right),
                _ => {
                    return Err(RuntimeError {
                        message: "arithmetic and comparison operators expect numbers".into(),
                    })
                }
            };
            let value = match operator {
                BinaryOp::Subtract => Value::Number(left - right),
                BinaryOp::Multiply => Value::Number(left * right),
                BinaryOp::Divide => {
                    if right == 0.0 {
                        return Err(RuntimeError {
                            message: "division by zero".into(),
                        });
                    }
                    Value::Number(left / right)
                }
                BinaryOp::Remainder => Value::Number(left % right),
                BinaryOp::Less => Value::Bool(left < right),
                BinaryOp::LessEqual => Value::Bool(left <= right),
                BinaryOp::Greater => Value::Bool(left > right),
                BinaryOp::GreaterEqual => Value::Bool(left >= right),
                _ => unreachable!(),
            };
            Ok(value)
        }
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Null => false,
        Value::Number(value) => *value != 0.0,
        Value::String(value) => !value.is_empty(),
        Value::Function(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cobra_lexer_quantum::tokenize;
    use cobra_parser_ast::parse;

    #[test]
    fn evaluates_functions_and_arithmetic() {
        let program = parse(
            tokenize("fn add(a, b) { return a + b; } let answer = add(20, 22); add(20, 22);")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(interpret(&program).unwrap().to_string(), "42");
    }
}
