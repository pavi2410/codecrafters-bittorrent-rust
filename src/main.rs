use serde_json;
use std::env;
use std::fmt;

// Available if you need it!
use serde_bencode::{de, value::Value};

fn display(value: &Value) {
    match value {
        Value::Bytes(bytes) => println!(f, "{:?}", bytes),
        Value::Int(num) => println!(f, "{}", num),
        Value::List(list) => println!(f, "{:?}", list),
        Value::Dict(dict) => println!(f, "{:?}", dict),
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value: Value = de::from_str(encoded_value).unwrap();
        display(&decoded_value);
    } else {
        println!("unknown command: {}", args[1])
    }
}
