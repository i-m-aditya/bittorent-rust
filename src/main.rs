use anyhow::Error;
use clap::{Parser, Subcommand};
use std::{env, fs::File, io::Read, path::Path};

use crate::{hasher::bytes_to_hex, parser::Parser as OtherParser};
use sha1::{Digest, Sha1};

mod hasher;
mod parser;
// Available if you need it!
use serde_bencode;

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

            let result = OtherParser::new().parse_torrent_file(&contents).unwrap();

            println!("Tracker URL: {}", result.announce);
            println!("Length: {}", result.info.length);
            let mut hasher = Sha1::default();
            hasher.update(serde_bencode::to_bytes(&result.info).unwrap());
            let res = hasher.finalize();
            println!("Info Hash: {:x}", res);

            println!("Piece Length: {}", result.info.piece_length);

            println!("Piece Hashes:");

            for piece in result.info.pieces.chunks(20) {
                println!("{}", bytes_to_hex(piece));
            }
        }
        Commands::Peers { path } => {
            let filepath = env::current_dir()?.join(Path::new(&path));

            let mut file = File::open(filepath).unwrap();

            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();

            let result = OtherParser::new().parse_torrent_file(&contents).unwrap();

            println!("Tracker URL: {}", result.announce);
            println!("Length: {}", result.info.length);
            let mut hasher = Sha1::default();
            hasher.update(serde_bencode::to_bytes(&result.info).unwrap());

            let info_hash = hasher
                .finalize()
                .iter()
                .map(|b| format!("%{:02x}", b))
                .collect::<Vec<String>>()
                .join("");

            println!("Info Hash: {}", info_hash);

            let client = reqwest::blocking::Client::new();

            let url = format!("{}?info_hash={}", result.announce, info_hash);

            let req = client
                .get(url)
                .query(&[
                    ("peer_id", String::from("-TR2940-5f2b3b3b3b3b")),
                    ("port", String::from("6881")),
                    ("uploaded", String::from("0")),
                    ("downloaded", String::from("0")),
                    ("left", result.info.length.to_string()),
                    ("compact", String::from("1")),
                ])
                .build()?;
            let response = client.execute(req).unwrap().bytes().unwrap();

            println!("Response: {:?}", response);
        }
    }

    // if command == "decode" {
    //     // You can use print statements as follows for debugging, they'll be visible when running tests.
    //     // println!("Logs from your program will appear here!");

    //     // Uncomment this block to pass the first stage
    //     let encoded_value = &args[2];
    //     let (decoded_value, _) = decode_bencoded_value(encoded_value, 0);
    //     println!("{}", decoded_value.to_string());
    // } else if command == "info" {
    //     let current_dir = env::current_dir().unwrap();
    //     let filename = args[2].clone();
    //     let filepath = current_dir.join(Path::new(&filename));

    //     let mut file = File::open(filepath).unwrap();

    //     let mut contents = Vec::new();
    //     file.read_to_end(&mut contents).unwrap();

    //     let result = Parser::new().parse_torrent_file(&contents).unwrap();

    //     println!("Tracker URL: {}", result.announce);
    //     println!("Length: {}", result.info.length);
    //     let mut hasher = Sha1::default();
    //     hasher.update(serde_bencode::to_bytes(&result.info).unwrap());
    //     let res = hasher.finalize();
    //     println!("Info Hash: {:x}", res);

    //     println!("Piece Length: {}", result.info.piece_length);

    //     println!("Piece Hashes:");

    //     for piece in result.info.pieces.chunks(20) {
    //         println!("{}", bytes_to_hex(piece));
    //     }
    // } else if command == "peers" {
    //     let current_dir = env::current_dir().unwrap();
    //     let filename = args[2].clone();
    //     let filepath = current_dir.join(Path::new(&filename));

    //     let mut file = File::open(filepath).unwrap();

    //     let mut contents = Vec::new();
    //     file.read_to_end(&mut contents).unwrap();

    //     let result = Parser::new().parse_torrent_file(&contents).unwrap();

    //     println!("Tracker URL: {}", result.announce);
    //     println!("Length: {}", result.info.length);
    //     let mut hasher = Sha1::default();
    //     hasher.update(serde_bencode::to_bytes(&result.info).unwrap());

    //     let info_hash = hasher
    //         .finalize()
    //         .iter()
    //         .map(|b| format!("%{:02x}", b))
    //         .collect::<Vec<String>>()
    //         .join("");

    //     println!("Info Hash: {}", info_hash);

    //     let client = reqwest::blocking::Client::new();

    //     let url = format!("{}?info_hash={}", result.announce, info_hash);

    //     let req = client
    //         .get(url)
    //         .query(&[
    //             ("peer_id", String::from("-TR2940-5f2b3b3b3b3b")),
    //             ("port", String::from("6881")),
    //             ("uploaded", String::from("0")),
    //             ("downloaded", String::from("0")),
    //             ("left", result.info.length.to_string()),
    //             ("compact", String::from("1")),
    //         ])
    //         .build()?;
    //     let response = client.execute(req).unwrap().bytes().unwrap();

    //     println!("Response: {:?}", response);
    // }
    Ok(())
}
