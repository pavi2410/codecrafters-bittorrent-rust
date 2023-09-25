use serde_json;
use std::env;
use std::fmt;

// Available if you need it!
use serde_bencode::{de, value::Value};

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Bytes(bytes) => write!(f, "{:?}", bytes),
            Value::Int(num) => write!(f, "{}", num),
            Value::List(list) => write!(f, "{:?}", list),
            Value::Dict(dict) => write!(f, "{:?}", dict),
        }
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value: Value = de::from_str(encoded_value).unwrap();
        println!("{}", decoded_value);
    } else {
        println!("unknown command: {}", args[1])
    }
}
