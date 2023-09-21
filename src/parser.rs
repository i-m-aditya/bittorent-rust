use std::{env, fs::File, io::Read, net::Ipv4Addr, path::Path};

use anyhow::{anyhow, Error, Ok, Result};
use serde::{Deserialize, Serialize};

use crate::{hasher::bytes_to_hex_url_encoded, request::TrackerResponse};
#[derive(Debug, Default)]
pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn parse_torrent_file(&mut self, input: &[u8]) -> Result<TorrentFile> {
        serde_bencode::from_bytes(input).map_err(|e| anyhow!("Failed to parse input: {}", e))
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TorrentFile {
    pub announce: String,
    pub info: TorrentInfo,
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TorrentInfo {
    pub length: u64,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
}

impl TorrentFile {
    pub fn parse_file_from_path(path: &String) -> Result<TorrentFile, Error> {
        let filepath = env::current_dir()?.join(Path::new(path));

        let mut file = File::open(filepath).unwrap();

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        let torrent_file = Parser::new().parse_torrent_file(&contents).unwrap();
        Ok(torrent_file)
    }

    pub fn discover_peers(&self) -> Result<Vec<(Ipv4Addr, u16)>, Error> {
        let client = reqwest::blocking::Client::new();

        let url_encoded_info_hash =
            bytes_to_hex_url_encoded(&serde_bencode::to_bytes(&self.info).unwrap());
        let url = format!("{}?info_hash={}", self.announce, url_encoded_info_hash);

        let req = client
            .get(url)
            .query(&[
                ("peer_id", String::from("-TR2940-5f2b3b3b3b3b")),
                ("port", String::from("6881")),
                ("uploaded", String::from("0")),
                ("downloaded", String::from("0")),
                ("left", self.info.length.to_string()),
                ("compact", String::from("1")),
            ])
            .build()?;
        let response = client.execute(req).unwrap().bytes().unwrap();

        let tracker_response = serde_bencode::from_bytes::<TrackerResponse>(&response)?;

        let mut peers = Vec::<(Ipv4Addr, u16)>::new();
        for peer in tracker_response.peers.chunks(6) {
            let mut ip = [0u8; 4];
            ip.copy_from_slice(&peer[..4]);
            let port = u16::from_be_bytes([peer[4], peer[5]]);
            peers.push((Ipv4Addr::from(ip), port));
        }
        Ok(peers)
    }
}
