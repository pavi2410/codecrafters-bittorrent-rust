use clap::{Parser, Subcommand};
use hex;
use serde::{Deserialize, Serialize};
use serde_bencode::{de, value::Value as BencodeValue};
use serde_bytes::ByteBuf;
use serde_json::{Map, Value as JsonValue};
use sha1::{Digest, Sha1};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::path::PathBuf;

const BLOCK_SIZE: usize = 16 * 1024;
const MY_PEER_ID: &str = "00112233445566778899";

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

// #[derive(Debug, Deserialize, Serialize)]
// struct Peer {
//     ip: Ipv4Addr,
//     port: u16,
// }

#[derive(Debug)]
enum PeerMessage {
    Unchoke,
    Interested,
    Bitfield,
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
}

impl PeerMessage {
    fn read_from_stream(stream: &mut TcpStream) -> Self {
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).unwrap();
        let length = u32::from_be_bytes(buf);

        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf).unwrap();
        let message_id = buf[0];

        println!("Length: {}", length);
        println!("Message ID: {}", message_id);

        let payload = if length > 1 {
            let mut buf = vec![0u8; length as usize - 1];
            stream.read_exact(&mut buf).unwrap();
            buf
        } else {
            vec![]
        };

        match message_id {
            1 => PeerMessage::Unchoke,
            2 => PeerMessage::Interested,
            5 => PeerMessage::Bitfield,
            6 => PeerMessage::Request {
                index: 0,
                begin: 0,
                length: 0,
            },
            7 => PeerMessage::Piece {
                index: u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]),
                begin: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
                block: payload[8..].to_vec(),
            },
            _ => panic!("Unknown message id: {}", message_id),
        }
    }

    fn write_to_stream(&self, stream: &mut TcpStream) {
        match &self {
            PeerMessage::Unchoke => {
                stream.write(&[0, 0, 0, 1, 1]).unwrap();
            }
            PeerMessage::Interested => {
                stream.write(&[0, 0, 0, 1, 2]).unwrap();
            }
            PeerMessage::Bitfield => {
                stream.write(&[0, 0, 0, 1, 5]).unwrap();
            }
            PeerMessage::Request {
                index,
                begin,
                length,
            } => {
                stream.write(&(97u32.to_be_bytes())).unwrap();
                stream.write(&[6]).unwrap();
                stream.write(&index.to_be_bytes()).unwrap();
                stream.write(&begin.to_be_bytes()).unwrap();
                stream.write(&length.to_be_bytes()).unwrap();
            }
            PeerMessage::Piece {
                index,
                begin,
                block,
            } => {
                stream.write(&[0, 0, 0, 9, 7]).unwrap();
                stream.write(&index.to_be_bytes()).unwrap();
                stream.write(&begin.to_be_bytes()).unwrap();
                stream.write(&block).unwrap();
            }
        }
    }
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
    fn from_file(file_name: &PathBuf) -> Result<Torrent, String> {
        let file_buf = std::fs::read(file_name).unwrap();

        let torrent = de::from_bytes::<Torrent>(&file_buf).unwrap();

        Ok(torrent)
    }
}

impl Info {
    fn get_info_hash(&self) -> [u8; 20] {
        let mut hasher = Sha1::new();
        hasher.update(serde_bencode::to_bytes(&self).unwrap());
        hasher.finalize().into()
    }
}

impl TrackerRequest {
    fn build_tracker_url(&self, announce_endpoint: String) -> String {
        format!(
            "{}?info_hash={}&{}",
            announce_endpoint,
            self.info_hash.clone(),
            serde_urlencoded::to_string(self).unwrap()
        )
    }
}

impl TrackerResponse {
    fn get_peers(&self) -> Vec<SocketAddrV4> {
        self.peers
            .chunks(6)
            .map(|chunk| {
                let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                let port = u16::from_be_bytes([chunk[4], chunk[5]]);

                SocketAddrV4::new(ip, port)
            })
            .collect::<Vec<_>>()
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
    #[command(name = "download_piece")]
    DownloadPiece {
        #[arg(short = 'o', value_name = "FILE")]
        output_file_name: PathBuf,

        #[arg(value_name = "FILE")]
        file_name: PathBuf,

        piece_index: usize,
    },
    Download {
        #[arg(short = 'o', value_name = "FILE")]
        output_file_name: PathBuf,

        #[arg(value_name = "FILE")]
        file_name: PathBuf,
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
            let torrent = Torrent::from_file(file_name).unwrap();

            let info_hash = torrent.info.get_info_hash();

            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", torrent.info.length);
            println!("Info Hash: {}", hex::encode(&info_hash));
            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes:");
            for piece in torrent.info.pieces.chunks(20) {
                println!("{}", hex::encode(piece));
            }
        }

        Some(Commands::Peers { file_name }) => {
            let torrent = Torrent::from_file(file_name).unwrap();

            let info_hash = torrent.info.get_info_hash();

            let tracker_options = TrackerRequest {
                info_hash: urlencode_bytes(&info_hash),
                peer_id: "00112233445566778899".to_string(),
                port: 6881,
                uploaded: 0,
                downloaded: 0,
                left: torrent.info.length,
                compact: 1,
            };

            let tracker_url = tracker_options.build_tracker_url(torrent.announce);

            let resp = reqwest::blocking::get(tracker_url)
                .unwrap()
                .bytes()
                .unwrap();

            let tracker_response = de::from_bytes::<TrackerResponse>(&resp).unwrap();

            for peer in tracker_response.get_peers() {
                println!("{}", peer);
            }
        }

        Some(Commands::Handshake {
            file_name,
            peer_endpoint,
        }) => {
            let torrent = Torrent::from_file(file_name).unwrap();

            let info_hash = torrent.info.get_info_hash();

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

        Some(Commands::DownloadPiece {
            output_file_name,
            file_name,
            piece_index,
        }) => {
            let torrent = Torrent::from_file(file_name).unwrap();

            let info_hash = torrent.info.get_info_hash();

            let tracker_options = TrackerRequest {
                info_hash: urlencode_bytes(&info_hash),
                peer_id: MY_PEER_ID.to_string(),
                port: 6881,
                uploaded: 0,
                downloaded: 0,
                left: torrent.info.length,
                compact: 1,
            };

            let tracker_url = tracker_options.build_tracker_url(torrent.announce);

            println!("{}", tracker_url);

            let resp = reqwest::blocking::get(tracker_url)
                .unwrap()
                .bytes()
                .unwrap();

            println!("{:?}", resp);

            let tracker_response = de::from_bytes::<TrackerResponse>(&resp).unwrap();

            let peer = tracker_response.get_peers()[0];

            let mut stream = TcpStream::connect(peer).unwrap();

            // handshake send
            stream.write(&[19]).unwrap();
            stream.write(b"BitTorrent protocol").unwrap();
            stream.write(&[0; 8]).unwrap();
            stream.write(&info_hash).unwrap();
            stream.write(MY_PEER_ID.as_bytes()).unwrap();

            // handshake receive
            let mut buf = [0u8; 68];
            stream.read_exact(&mut buf).unwrap();
            println!("Peer ID: {}", hex::encode(&buf[48..]));

            // bitfield receive
            PeerMessage::read_from_stream(&mut stream);

            // interested send
            PeerMessage::Interested.write_to_stream(&mut stream);

            // unchoke receive
            PeerMessage::read_from_stream(&mut stream);

            println!("Unchoked");

            // request send and piece receive

            let mut out_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(output_file_name)
                .unwrap();


            let total_pieces = (torrent.info.length as f32 / torrent.info.piece_length as f32).ceil() as usize;

            let piece_length = if *piece_index == total_pieces - 1 {
                torrent.info.length % torrent.info.piece_length
            } else {
                torrent.info.piece_length
            };

            let piece = download_piece(&mut stream, *piece_index, piece_length);

            out_file.write(&piece).unwrap();

            println!(
                "Piece {} downloaded to {}",
                piece_index,
                output_file_name.display()
            );
        }

        Some(Commands::Download {
            output_file_name,
            file_name,
        }) => {

            let torrent = Torrent::from_file(file_name).unwrap();

            let info_hash = torrent.info.get_info_hash();

            let tracker_options = TrackerRequest {
                info_hash: urlencode_bytes(&info_hash),
                peer_id: MY_PEER_ID.to_string(),
                port: 6881,
                uploaded: 0,
                downloaded: 0,
                left: torrent.info.length,
                compact: 1,
            };

            let tracker_url = tracker_options.build_tracker_url(torrent.announce);

            println!("{}", tracker_url);

            let resp = reqwest::blocking::get(tracker_url)
                .unwrap()
                .bytes()
                .unwrap();

            println!("{:?}", resp);

            let tracker_response = de::from_bytes::<TrackerResponse>(&resp).unwrap();

            let peer = tracker_response.get_peers()[0];

            let mut stream = TcpStream::connect(peer).unwrap();

            // handshake send
            stream.write(&[19]).unwrap();
            stream.write(b"BitTorrent protocol").unwrap();
            stream.write(&[0; 8]).unwrap();
            stream.write(&info_hash).unwrap();
            stream.write(MY_PEER_ID.as_bytes()).unwrap();

            // handshake receive
            let mut buf = [0u8; 68];
            stream.read_exact(&mut buf).unwrap();
            println!("Peer ID: {}", hex::encode(&buf[48..]));

            // bitfield receive
            PeerMessage::read_from_stream(&mut stream);

            // interested send
            PeerMessage::Interested.write_to_stream(&mut stream);

            // unchoke receive
            PeerMessage::read_from_stream(&mut stream);

            println!("Unchoked");

            // request send and piece receive

            let mut out_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(output_file_name)
                .unwrap();


            let total_pieces = (torrent.info.length as f32 / torrent.info.piece_length as f32).ceil() as usize;

            for piece_index in 0..total_pieces {
                let piece_size = if piece_index == total_pieces - 1 {
                    torrent.info.length % torrent.info.piece_length
                } else {
                    torrent.info.piece_length
                };

                let piece = download_piece(&mut stream, piece_index, piece_size);

                let piece_offset = piece_index * torrent.info.piece_length;

                out_file.seek(SeekFrom::Start())
                out_file.write(piece).unwrap();
            }

            println!(
                "Downloaded {} to {}",
                file_name.display(),
                output_file_name.display()
            );
        }

        None => {
            println!("Unknown command");
        }
    }
}

fn urlencode_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("%{:02X}", b))
        .collect::<String>()
}

fn download_piece(stream: &mut TcpStream, piece_index: usize, piece_size: usize) -> Vec<u8> {
    let total_blocks = (piece_size as f32 / BLOCK_SIZE as f32).ceil() as usize;

    println!("Piece length: {}", piece_size);
    println!("Expecting {} blocks", total_blocks);

    for i in 0..total_blocks {
        let request = PeerMessage::Request {
            index: piece_index as u32,
            begin: (i * BLOCK_SIZE) as u32,
            length: if i == total_blocks - 1 {
                piece_size - i * BLOCK_SIZE
            } else {
                BLOCK_SIZE
            } as u32,
        };
        println!("{} Requesting {:?}", i, request);
        request.write_to_stream(stream);
    }

    let mut piece: Vec<u8> = vec![0; piece_size];

    for i in 0..total_blocks {
        let block = PeerMessage::read_from_stream(stream);
        match block {
            PeerMessage::Piece { begin, block, .. } => {
                println!("{} Received block at {}", i, begin);

                let begin = begin as usize;

                for i in 0..block.len() {
                    piece[begin + i] = block[i];
                }
            }
            _ => panic!("Expected piece"),
        }
    }

    // TODO: verify hash

    return piece;
}