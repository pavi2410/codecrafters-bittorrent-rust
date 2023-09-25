use serde_json;
use std::env;

// Available if you need it!
use serde_bencode::{de, value::Value};

fn display(value: &Value) {
    match value {
        Value::Bytes(bytes) => println!("{:?}", String::from_utf8_lossy(bytes)),
        Value::Int(num) => println!("{}", num),
        Value::List(list) => println!("{:?}", list),
        Value::Dict(dict) => println!("{:?}", dict),
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
