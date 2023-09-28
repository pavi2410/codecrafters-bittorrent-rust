use hex;
use serde::{Deserialize, Serialize};
use serde_bencode::{de, value::Value as BencodeValue};
use serde_bytes::ByteBuf;
use serde_json::{Map, Value as JsonValue};
use sha1::{Digest, Sha1};
use std::env;
use std::net::Ipv4Addr;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TrackerRequest {
    #[serde(skip_serializing)]
    info_hash: String,
    peer_id: String,
    port: u16,
    uploaded: usize,
    downloaded: usize,
    left: usize,
    compact: u8,
}

#[derive(Debug, Deserialize, Serialize)]
struct TrackerResponse {
    // interval: usize,
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>,
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

        println!("Info Hash: {}", hex::encode(&info_hash(&torrent.info)));

        let tracker_options = TrackerRequest {
            info_hash: urlencode_bytes(&info_hash(&torrent.info)),
            peer_id: "00112233445566778899".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: torrent.info.length,
            compact: 1,
        };

        let tracker_url = format!(
            "{}?info_hash={}&{}",
            torrent.announce,
            tracker_options.info_hash.clone(),
            serde_urlencoded::to_string(tracker_options).unwrap()
        );

        println!("{}", tracker_url);

        let resp = reqwest::blocking::get(tracker_url)
            .unwrap()
            .bytes()
            .unwrap();

        println!("{:?}", resp);

        let tracker_response = de::from_bytes::<TrackerResponse>(&resp).unwrap();

        println!("{:#?}", tracker_response.peers);

        // for peers in tracker_response.peers.chunks(6) {
        //     println!("{}:{:?}", Ipv4Addr::from(peers[..4]), peers[4..]);
        //     println!("peer: {:?}", peers);
        // }
    } else {
        println!("unknown command: {}", args[1])
    }
}

fn info_hash(info: &Info) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(serde_bencode::to_bytes(&info).unwrap());
    hasher.finalize().into()
}

fn urlencode_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("%{:X}", b))
        .collect::<String>()
}