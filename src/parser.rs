use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
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
impl TorrentInfo {
    pub fn hash(&self) -> Result<Vec<u8>> {
        let mut hasher = Sha1::default();
        hasher.update(serde_bencode::to_bytes(self)?);
        Ok(hasher.finalize().to_vec())
    }

    pub fn sha1_hashes_to_hex(&self) -> Result<Vec<String>> {
        let mut hasher = Sha1::default();
        let sha1_hashes: Vec<String> = self
            .pieces
            .chunks(20) // Assuming each SHA-1 hash is 20 bytes
            .map(|chunk| {
                hasher.update(chunk);
                let result = hasher.finalize_reset();
                format!("{:x}", result)
            })
            .collect();
        Ok(sha1_hashes)
    }
}
