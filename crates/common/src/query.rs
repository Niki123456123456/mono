use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pair;
use pest::iterators::Pairs;

#[derive(Parser)]
#[grammar = "quer.pest"]
pub struct QueryParser;

#[derive(PartialEq, Eq, Debug, Clone)]
pub  enum Value {
    Number(i64),
    String(String),
    Array(Vec<Value>),
    None
}
impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(s) => s
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
            Value::None => String::new(),
        }
    }
}

impl std::cmp::PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
            (Value::String(a), Value::String(b)) => a.partial_cmp(b),
            (Value::Array(a), Value::Array(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl std::cmp::Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a.cmp(b),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Array(a), Value::Array(b)) => a.cmp(b),
            (Value::None, _) => std::cmp::Ordering::Less,
            (_, Value::None) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Query {
    Logical {
        left: Box<Query>,
        op: LogicalOperator,
        right: Box<Query>,
    },
    Comparison {
        left: String,
        op: Operator,
        right: Value,
    },
}

#[derive(Debug, Clone)]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Eq,
    Gt,
    Lt,
    Ge,
    Le,
    Ne,
}

pub fn try_parse_query(input: &str) -> Option<Query> {
    // Attempt to parse the input starting at the expression rule
    let mut pairs = QueryParser::parse(Rule::expression, input).ok()?;

    // Get the first pair if any, else return None
    let expr_pair = pairs.next()?;

    // Convert the parse tree to Expression
    Some(build_ast(expr_pair))
}

fn build_ast(pair: Pair<Rule>) -> Query {
    match pair.as_rule() {
        Rule::expression => build_ast(pair.into_inner().next().unwrap()),

        Rule::logical_or => {
            let mut inner = pair.into_inner();
            let first = build_ast(inner.next().unwrap());
            let mut result = first;

            while let Some(op_pair) = inner.next() {
                let right = build_ast(inner.next().unwrap());
                result = Query::Logical {
                    left: Box::new(result),
                    op: LogicalOperator::Or,
                    right: Box::new(right),
                };
            }
            result
        }

        Rule::logical_and => {
            let mut inner = pair.into_inner();
            let first = build_ast(inner.next().unwrap());
            let mut result = first;

            while let Some(op_pair) = inner.next() {
                let right = build_ast(inner.next().unwrap());
                result = Query::Logical {
                    left: Box::new(result),
                    op: LogicalOperator::And,
                    right: Box::new(right),
                };
            }
            result
        }

        Rule::logical_primary => {
            let inner = pair.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::comparison => build_ast(inner),
                Rule::expression => build_ast(inner), // parenthesized expression
                _ => unreachable!(),
            }
        }

        Rule::comparison => {
            let mut inner = pair.into_inner();
            let left = inner.next().unwrap().as_str().to_string();
            let op = parse_operator(inner.next().unwrap().as_str());
            let right = parse_value(inner.next().unwrap());
            Query::Comparison { left, op, right }
        }

        _ => unreachable!("Unexpected rule: {:?}", pair.as_rule()),
    }
}

fn parse_operator(op_str: &str) -> Operator {
    match op_str {
        "=" => Operator::Eq,
        ">" => Operator::Gt,
        "<" => Operator::Lt,
        ">=" => Operator::Ge,
        "<=" => Operator::Le,
        "!=" => Operator::Ne,
        _ => panic!("Unknown operator {}", op_str),
    }
}

fn parse_value(pair: Pair<Rule>) -> Value {
    match pair.as_rule() {
        Rule::number => {
            let n = pair.as_str().parse::<i64>().unwrap();
            Value::Number(n)
        }
        Rule::string => {
            // remove quotes from string literal
            let s = pair.as_str();
            let s = &s[1..s.len() - 1];
            Value::String(s.to_string())
        }
        Rule::identifier => Value::String(pair.as_str().to_string()), // treat identifiers as string values here
        _ => unreachable!("Unexpected value rule {:?}", pair.as_rule()),
    }
}