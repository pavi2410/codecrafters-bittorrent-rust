use std::env;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use serde_bencode::{de, value::Value as BencodeValue};
use serde_bytes::ByteBuf;
use sha1::{Sha1, Digest};
use hex;

#[derive(Debug, Deserialize, Serialize)]
struct Torrent {
    announce: String,
    info: Info,
}

#[derive(Debug, Deserialize, Serialize)]
struct Info {
    length: usize,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: usize,
    pieces: ByteBuf,
}


fn to_json(value: &BencodeValue) -> JsonValue {
    match value {
        BencodeValue::Bytes(bytes) => JsonValue::String(String::from_utf8_lossy(bytes).to_string()),
        BencodeValue::Int(num) => JsonValue::Number(num.to_owned().into()),
        BencodeValue::List(list) => JsonValue::Array(list.iter().map(|v| to_json(v)).collect()),
        BencodeValue::Dict(dict) => {
            let mut json_dict = Map::new();
            for (key, val) in dict.iter() {
                let key = String::from_utf8(key.clone()).unwrap().to_string();
                let val = to_json(val);
                json_dict.insert(key, val);
            }
            JsonValue::Object(json_dict)
        }
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value: BencodeValue = de::from_str(encoded_value).unwrap();
        println!("{}", to_json(&decoded_value));
    } else if command == "info" {
        let file_name = &args[2];

        let file_buf = std::fs::read(file_name).unwrap();

        let torrent = de::from_bytes::<Torrent>(&file_buf).unwrap();

        println!("Tracker URL: {}", torrent.announce);
        println!("Length: {}", torrent.info.length);
        println!("Info Hash: {}", info_hash(&torrent.info));
    } else {
        println!("unknown command: {}", args[1])
    }
}

fn info_hash(info: &Info) -> String {
    let mut hasher = Sha1::new();
    hasher.update(serde_bencode::to_bytes(&info).unwrap());
    hex::encode(hasher.finalize())
}
