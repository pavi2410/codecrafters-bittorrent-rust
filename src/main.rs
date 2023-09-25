use serde_json;
use std::env;

// Available if you need it!
use serde_bencode::{de, value::Value};

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value = de::from_str(encoded_value).unwrap();
        println!("{}", decoded_value);
    } else {
        println!("unknown command: {}", args[1])
    }
}
