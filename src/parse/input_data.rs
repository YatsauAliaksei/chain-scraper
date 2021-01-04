use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use serde::export::Formatter;

#[derive(Serialize, Deserialize, Debug)]
pub struct InputData {
    method_name: String,
    args: HashMap<String, Type>,
}

impl InputData {
    pub fn method_name(&self) -> &str {
        self.method_name.as_str()
    }

    pub fn args(&self) -> &HashMap<String, Type> {
        &self.args
    }
}

#[derive(Serialize, Deserialize)]
pub enum Type {
    INT(u64),
    BOOL(bool),
    STRING(String),
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::INT(i) => write!(f, "{}", i),
            Type::BOOL(i) => write!(f, "{}", i),
            Type::STRING(i) => write!(f, "{}", i),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::INT(i) => write!(f, "{}", i),
            Type::BOOL(i) => write!(f, "{}", i),
            Type::STRING(i) => write!(f, "{}", i),
        }
    }
}

impl InputData {
    pub fn new(method_name: &str, args: HashMap<String, Type>) -> Self {
        InputData {
            method_name: method_name.into(),
            args,
        }
    }
}