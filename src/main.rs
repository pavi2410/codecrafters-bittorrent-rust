use serde_json;
use std::env;

// Available if you need it!
use serde_bencode::{de, value::Value};

fn to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Bytes(bytes) => serde_json::Value::String(String::from_utf8_lossy(bytes).to_string()),
        Value::Int(num) => serde_json::Value::Number(num.to_owned().into()),
        Value::List(list) => serde_json::Value::Array(list.to_owned().map(to_json).into()),
        Value::Dict(dict) => serde_json::Value::Object(dict.to_owned().into()),
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value: Value = de::from_str(encoded_value).unwrap();
        println!("{}", to_json(&decoded_value));
    } else {
        println!("unknown command: {}", args[1])
    }
}
