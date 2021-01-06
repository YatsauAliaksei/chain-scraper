use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use serde::export::Formatter;
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct InputData {
    method_name: String,
    args: Map<String, Value>,
}

impl InputData {
    pub fn method_name(&self) -> &str {
        self.method_name.as_str()
    }

    pub fn args(&self) -> &Map<String, Value> {
        &self.args
    }
}

/*#[derive(Serialize, Deserialize)]
pub enum Type {
    INT(u64),
    BOOL(bool),
    STRING(String),
}
*/
/*impl Debug for Type {
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
*/
impl InputData {
    pub fn new(method_name: &str, args: Map<String, Value>) -> Self {
        InputData {
            method_name: method_name.into(),
            args,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Map;

    use super::*;

    #[test]
    fn serialize() {
        let mut args = Map::new();
        // let orig = r#"{"id":132,"name":"Alex"}"#;
        let orig = r#"{"id":132,"name":"Alex"}"#;

        println!("O: {}", orig);

        args.insert("one".into(), Value::from(1));
        args.insert("one".into(), Value::from("Hello World"));

        // args.insert("clientData".into(), Value::from("{\"tax\":132,\"number\":\"UUID-1234\"}"));



        let mut orig_value: Map<String, Value> = serde_json::from_str(orig).expect("Orig value failed");
        // println!("STR: {}", orig_value.as_str().unwrap());
        println!("V: {:?}", orig_value);
        // println!("VS: {}", orig_value.to_string());

        // let map = orig_value.as_object_mut().unwrap();
        // println!("Map: {:?}", map);


        let id = InputData {
            method_name: "submit".into(),
            args,
        };

        let json = serde_json::to_string(&id).unwrap();
        println!("{}", json);
    }
}