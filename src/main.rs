use anyhow::{anyhow, Error};
use clap::{Args, Parser, Subcommand};
use std::{
    env,
    fs::File,
    io::{Read, Write},
    net::{SocketAddr, SocketAddrV4},
    path::Path,
    str::FromStr,
};
use tcp::Connection;
mod tcp;

use std::net::Ipv4Addr;

use crate::{hasher::hash_bytes_and_hex, parser::Parser as OtherParser};

mod hasher;
mod parser;
mod request;
// Available if you need it!
use serde_bencode;

use hasher::{bytes_to_hex, bytes_to_hex_url_encoded, hash_bytes};
use request::TrackerResponse;

fn find_e_for_index(s: &str, index: usize) -> usize {
    let mut count = 1;
    let mut i = index + 1;

    while i < s.len() {
        if s.chars().nth(i as usize).unwrap() == 'e' {
            count -= 1;
        } else if s.chars().nth(i as usize).unwrap() == 'l'
            || s.chars().nth(i as usize).unwrap() == 'd'
            || s.chars().nth(i as usize).unwrap() == 'i'
        {
            count += 1;
        }

        if count == 0 {
            return i;
        }

        i += 1;
    }

    return 0;
}

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[clap(name = "decode")]
    Decode { string: String },
    #[clap(name = "info")]
    Info { path: String },
    #[clap(name = "peers")]
    Peers { path: String },
    #[clap(name = "handshake")]
    Handshake { path: String, url: String },
    #[clap(name = "download_piece")]
    DownloadPiece(DownloadPieceArgs),
}

#[derive(Args, Debug)]
pub struct DownloadPieceArgs {
    #[clap(short, long, help = "Peer address")]
    address: Option<SocketAddrV4>,
    #[clap(short, long, help = "File path")]
    output: Option<String>,
    #[clap(short, long, help = "Debug mode")]
    debug: Option<bool>,
    piece: usize,
}
#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str, index: usize) -> (serde_json::Value, usize) {
    // println!("encoded_value: {}", encoded_value);
    if encoded_value.chars().nth(index).unwrap().is_digit(10) {
        let parts: Vec<&str> = encoded_value[index..].split(":").collect();
        let num_string = parts[0].to_string();
        let num_integer = num_string.parse::<i32>().unwrap();

        let start = index + num_string.len() + 1;
        let end = start + num_integer as usize;

        // println!("start {}, end {}, len {}", start, end, encoded_value.len());
        if end > encoded_value.len() {
            return (
                serde_json::Value::String("".to_string()),
                encoded_value.len(),
            );
        }

        let decoded_string = &encoded_value[start..end];

        // println!("decoded string {}, end {}", decoded_string, end);
        return (serde_json::Value::String(decoded_string.to_string()), end);
    } else if encoded_value.chars().nth(index).unwrap() == 'i' {
        let e_position = find_e_for_index(encoded_value, index);

        let parsed_value = &encoded_value[index + 1..e_position];

        // println!("decoded string {}, end {}", parsed_value, e_position + 1);

        return (
            serde_json::Value::Number(parsed_value.parse::<i64>().unwrap().into()),
            e_position + 1,
        );
    } else if encoded_value.chars().nth(index).unwrap() == 'l' {
        let mut i = index + 1;

        // println!("i : {}", i);

        let mut lst: Vec<serde_json::Value> = Vec::new();

        while i < encoded_value.len() {
            if encoded_value.chars().nth(i).unwrap() == 'e' {
                break;
            } else {
                let (decoded_value, new_index) = decode_bencoded_value(encoded_value, i);
                // println!(
                //     "decoded_value {}, new_index {}",
                //     decoded_value.to_string(),
                //     new_index
                // );
                lst.push(decoded_value);
                i = new_index;
            }
        }

        // println!("decoded list {:?}, end {}", lst, i + 1);
        return (serde_json::Value::Array(lst), i + 1);
    } else if encoded_value.chars().nth(index).unwrap() == 'd' {
        // println!(" hello dict, index: {}, len {}", index, encoded_value.len());
        let mut i = index + 1;

        let mut dict: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

        while i < encoded_value.len() {
            if encoded_value.chars().nth(i).unwrap() == 'e' {
                break;
            } else {
                let (decoded_key, new_index) = decode_bencoded_value(encoded_value, i);
                let (decoded_value, new_index) = decode_bencoded_value(encoded_value, new_index);
                dict.insert(decoded_key.as_str().unwrap().to_string(), decoded_value);
                i = new_index;
            }
        }
        // println!("End dict {:?}, end {}", dict, i + 1);
        return (serde_json::Value::Object(dict), i + 1);
    } else {
        panic!("Not implemented")
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { string } => {
            let (decoded_value, _) = decode_bencoded_value(&string, 0);
            println!("{}", decoded_value.to_string());
        }
        Commands::Info { path } => {
            let filepath = env::current_dir()?.join(Path::new(&path));

            let mut file = File::open(filepath).unwrap();

            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();

            let torrent_file = OtherParser::new().parse_torrent_file(&contents).unwrap();

            println!("Tracker URL: {}", torrent_file.announce);
            println!("Length: {}", torrent_file.info.length);

            println!(
                "Info Hash: {}",
                hash_bytes_and_hex(&serde_bencode::to_bytes(&torrent_file.info)?)
            );

            println!("Piece Length: {}", torrent_file.info.piece_length);

            println!("Piece Hashes:");

            for piece in torrent_file.info.pieces.chunks(20) {
                println!("{}", bytes_to_hex(piece));
            }
        }
        Commands::Peers { path } => {
            let filepath = env::current_dir()?.join(Path::new(&path));

            let mut file = File::open(filepath).unwrap();

            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();

            let torrent_file = OtherParser::new().parse_torrent_file(&contents).unwrap();

            let info_hash = bytes_to_hex_url_encoded(&serde_bencode::to_bytes(&torrent_file.info)?);

            let client = reqwest::blocking::Client::new();

            let url = format!("{}?info_hash={}", torrent_file.announce, info_hash);

            let req = client
                .get(url)
                .query(&[
                    ("peer_id", String::from("-TR2940-5f2b3b3b3b3b")),
                    ("port", String::from("6881")),
                    ("uploaded", String::from("0")),
                    ("downloaded", String::from("0")),
                    ("left", torrent_file.info.length.to_string()),
                    ("compact", String::from("1")),
                ])
                .build()?;
            let response = client.execute(req).unwrap().bytes().unwrap();

            // println!("{:?}", response);

            let tracker_response = serde_bencode::from_bytes::<TrackerResponse>(&response)?;

            let mut peers = Vec::<(Ipv4Addr, u16)>::new();
            for peer in tracker_response.peers.chunks(6) {
                let mut ip = [0u8; 4];
                ip.copy_from_slice(&peer[..4]);
                let port = u16::from_be_bytes([peer[4], peer[5]]);
                peers.push((Ipv4Addr::from(ip), port));
            }

            for peer in peers {
                println!("{}:{}", peer.0, peer.1);
            }

            // let mut peers: Vec<String> = Vec::new();
        }
        Commands::Handshake { path, url } => {
            let filepath = env::current_dir()?.join(Path::new(&path));
            let mut file = File::open(filepath).unwrap();

            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();

            let torrent_file = OtherParser::new().parse_torrent_file(&contents).unwrap();

            let infohash = hash_bytes(&serde_bencode::to_bytes(&torrent_file.info)?);

            let mut connection = Connection::new(url);
            let peer_id = connection.handshake(&infohash.to_vec());

            println!("Peer ID: {}", peer_id);
        }
        Commands::DownloadPiece(download_piece_args) => {
            println!("Download Piece Args: {:?}", download_piece_args);
        }
    }

    Ok(())
}
