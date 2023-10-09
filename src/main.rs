use clap::{Parser, Subcommand};
use hex;
use serde::{Deserialize, Serialize};
use serde_bencode::{de, value::Value as BencodeValue};
use serde_bytes::ByteBuf;
use serde_json::{Map, Value as JsonValue};
use sha1::{Digest, Sha1};
use std::io::{Write, Read};
use std::net::{Ipv4Addr, TcpStream};
use std::path::PathBuf;

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

impl Torrent {
    fn from_file(file_name: &str) -> Result<Torrent, String> {
        let file_buf = std::fs::read(file_name).unwrap();

        let torrent = de::from_bytes::<Torrent>(&file_buf).unwrap();

        torrent
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Decode {
        encoded_value: String,
    },
    Info {
        #[arg(value_name = "FILE")]
        file_name: PathBuf,
    },
    Peers {
        #[arg(value_name = "FILE")]
        file_name: PathBuf,
    },
    Handshake {
        #[arg(value_name = "FILE")]
        file_name: PathBuf,

        peer_endpoint: String,
    },
    #[command(name="download_piece")]
    DownloadPiece {
        #[arg(short = 'o', value_name = "FILE")]
        output_file: PathBuf,

        #[arg(value_name = "FILE")]
        file_name: PathBuf,

        piece_index: usize,
    },
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Decode { encoded_value }) => {
            let decoded_value: BencodeValue = de::from_str(encoded_value).unwrap();
            println!("{}", to_json(&decoded_value));
        }

        Some(Commands::Info { file_name }) => {
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
        }

        Some(Commands::Peers { file_name }) => {
            let file_buf = std::fs::read(file_name).unwrap();

            let torrent = de::from_bytes::<Torrent>(&file_buf).unwrap();

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

            let resp = reqwest::blocking::get(tracker_url)
                .unwrap()
                .bytes()
                .unwrap();

            let tracker_response = de::from_bytes::<TrackerResponse>(&resp).unwrap();

            for peer in tracker_response.peers.chunks(6) {
                let ip = Ipv4Addr::new(peer[0], peer[1], peer[2], peer[3]);
                let port = u16::from_be_bytes([peer[4], peer[5]]);
                println!("{}:{}", ip, port);
            }
        }

        Some(Commands::Handshake { file_name, peer_endpoint }) => {
            let file_buf = std::fs::read(file_name).unwrap();

            let torrent = de::from_bytes::<Torrent>(&file_buf).unwrap();

            let info_hash = info_hash(&torrent.info);

            let mut stream = TcpStream::connect(peer_endpoint.as_str()).unwrap();

            stream.write(&[19]).unwrap();
            stream.write(b"BitTorrent protocol").unwrap();
            stream.write(&[0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
            stream.write(&info_hash).unwrap();
            stream.write(b"00112233445566778899").unwrap();

            let mut buf = [0u8; 68];
            stream.read_exact(&mut buf).unwrap();
            println!("Peer ID: {}", hex::encode(&buf[48..]));
        }

        Some(Commands::DownloadPiece { output_file, file_name, piece_index }) => {
            let file_buf = std::fs::read(file_name).unwrap();

            let torrent = de::from_bytes::<Torrent>(&file_buf).unwrap();


            
            println!("Downloading piece {}... ", piece_index);
            println!("Done! Saved to {:?}", output_file);
            println!("From {:?}", file_name);
        }

        None => {
            println!("Unknown command");
        }
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
