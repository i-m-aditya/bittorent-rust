use std::{
    io::{Read, Write},
    net::TcpStream,
};

use crate::hasher::bytes_to_hex;

pub struct Connection {
    pub stream: TcpStream,
}

impl Connection {
    pub fn new(address: String) -> Self {
        let stream = TcpStream::connect(address).expect("Connection Failed");
        Connection { stream }
    }

    pub fn handshake(&mut self, infohash: &Vec<u8>) -> String {
        // Construct a message
        let mut message = vec![19];
        message.extend(b"BitTorrent protocol"); // 19 bytes
        message.extend([0u8; 8]);
        message.extend(infohash);
        message.extend(b"00112233445566778899");

        let _ = self.stream.write(&message);

        let mut response = vec![0; message.len()];
        let _ = self.stream.read(&mut response);

        let response_peer_id = &response[response.len() - 20..];
        // println!("Peer ID: {}", bytes_to_hex(response_peer_id));
        return bytes_to_hex(response_peer_id);
    }
}
