use serde_json;
use std::env;

// Available if you need it!
use serde_bencode::{de, value::Value};

fn display(value: &Value) -> String {
    match value {
        Value::Bytes(bytes) => format!("{:?}", String::from_utf8_lossy(bytes)),
        Value::Int(num) => format!("{}", num),
        Value::List(list) => format!("{:?}", list.map(::display)),
        Value::Dict(dict) => format!("{:?}", dict),
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value: Value = de::from_str(encoded_value).unwrap();
        println!("{}", display(&decoded_value));
    } else {
        println!("unknown command: {}", args[1])
    }
}
