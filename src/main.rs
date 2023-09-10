use serde_json;
use std::{env, fs::File, io::Read, path::Path};

use crate::parser::Parser;
use sha1::{Digest, Sha1};

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

fn main() {
    let args: Vec<String> = env::args().collect();
    // println!("args: {:?}", args);
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // println!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let (decoded_value, _) = decode_bencoded_value(encoded_value, 0);
        println!("{}", decoded_value.to_string());
    } else if command == "info" {
        let current_dir = env::current_dir().unwrap();

        let filename = args[2].clone();

        let filepath = current_dir.join(Path::new(&filename));

        let mut file = File::open(filepath).unwrap();

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        let result = Parser::new().parse_torrent_file(&contents).unwrap();

        println!("Tracker URL: {}", result.announce);
        println!("Length: {}", result.info.length);
        let mut hasher = Sha1::default();
        hasher.update(serde_bencode::to_bytes(&result.info).unwrap());
        let res = hasher.finalize();
        println!("Info Hash: {:x}", res);
        // // println!("contents: {:?}", contents);

        // let binary_data_start = contents
        //     .iter()
        //     .position(|&x| x >= 128)
        //     .unwrap_or(contents.len());

        // // Convert the UTF-8 portion to a valid string
        // let utf8_text: String = contents[..binary_data_start]
        //     .iter()
        //     .map(|&byte| byte as char)
        //     .collect();

        // // Extract the binary data portion
        // let binary_data: &[u8] = &contents[binary_data_start..];

        // // // // Print the UTF-8 text
        // // println!("UTF-8 Text: {}", utf8_text);
        // // // // You can work with the binary data separately
        // // println!("Binary Data: {:?}", binary_data);

        // // println!("SHA1 Hash: {:x}", result);

        // let (decoded_value, _) = decode_bencoded_value(&utf8_text, 0);

        // // println!("{}", decoded_value.to_string());
        // let url = &decoded_value["announce"];
        // println!("Tracker URL: {}", url.as_str().unwrap());
        // let length = &decoded_value["info"]["length"];
        // println!("Length: {}", length);

        // let name = &decoded_value["info"]["name"];
        // let piece_length = &decoded_value["info"]["piece length"];
        // let pieces = binary_data;
        // let length = &decoded_value["info"]["length"];

        // let product = Product {
        //     length: length.as_u64().unwrap() as usize,
        //     name: name.as_str().unwrap().to_string(),
        //     piece_length: piece_length.as_u64().unwrap() as usize,
        //     pieces: pieces.to_vec(),
        // };

        // let bencoding = serde_bencode::to_string(&product).unwrap();

        // // println!("bencoding: {}", bencoding);

        // let mut hasher = Sha1::new();

        // hasher.update(&bencoding);
        // let result = hasher.finalize();
        // println!("Info Hash: {:x}", result);
    }
}
