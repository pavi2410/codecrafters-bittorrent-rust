use hex;
use serde::{Deserialize, Serialize};
use serde_bencode::{de, value::Value as BencodeValue};
use serde_bytes::ByteBuf;
use serde_json::{Map, Value as JsonValue};
use sha1::{Digest, Sha1};
use std::env;

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

#[derive(Debug, Deserialize, Serialize)]
struct TrackerResponse {
    interval: usize,
    peers: Vec<Peer>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Peer {
    ip: u32,
    port: u16,
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
        println!("Info Hash: {}", hex::encode(info_hash(&torrent.info)));
        println!("Piece Length: {}", torrent.info.piece_length);
        println!("Piece Hashes:");
        for piece in torrent.info.pieces.chunks(20) {
            println!("{}", hex::encode(piece));
        }
    } else if command == "peers" {
        let file_name = &args[2];

        let file_buf = std::fs::read(file_name).unwrap();

        let torrent = de::from_bytes::<Torrent>(&file_buf).unwrap();

        let tracker_options = &[
            ("info_hash", info_hash(&torrent.info)),
            ("peer_id", "00112233445566778899".to_string()),
            ("port", "6881".to_string()),
            ("uploaded", "0".to_string()),
            ("downloaded", "0".to_string()),
            ("left", torrent.info.length.to_string()),
            ("compact", "1".to_string()),
        ];
        let tracker_url = format!("{}?{}", torrent.announce, serde_urlencoded::to_string(tracker_options).unwrap());

        println!("{}", tracker_url);

        let resp = reqwest::blocking::get(tracker_url)
            .unwrap()
            .text()
            .unwrap();

        let tracker_response = de::from_str::<TrackerResponse>(&resp).unwrap();

        for peers in tracker_response.peers {
            println!("{}:{}", Ipv4Addr::from(peers.ip), peers.port);
        }
    } else {
        println!("unknown command: {}", args[1])
    }
}

fn info_hash(info: &Info) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(serde_bencode::to_bytes(&info).unwrap());
    hasher.finalize().into()
}
