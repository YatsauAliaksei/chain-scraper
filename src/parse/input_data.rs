use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct InputData {
    method_name: String,
    args: Map<String, Value>,
}

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
        let orig = r#"{"id":132,"name":"Alex"}"#;

        println!("O: {}", orig);

        args.insert("one".into(), Value::from(1));
        args.insert("one".into(), Value::from("Hello World"));

        let mut orig_value: Map<String, Value> = serde_json::from_str(orig).expect("Orig value failed");
        println!("V: {:?}", orig_value);

        let id = InputData {
            method_name: "submit".into(),
            args,
        };

        let json = serde_json::to_string(&id).unwrap();
        println!("{}", json);
    }
}