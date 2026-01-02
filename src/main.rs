#![allow(unused)]

use std::{
    io::{BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

use anyhow::bail;

use crate::datacoding::*;
use crate::rwbuf::{MyBufReader, MyBufWriter};

mod datacoding;
mod rwbuf;

const EXAMPLE_RESPONSE: &'static str = r#"{
    "version": {
        "name": "1.21.11",
        "protocol": 774
    },
    "players": {
        "max": 420,
        "online": 69,
        "sample": []
    },
    "description": {
        "text": "Server is down. Login to start it up!"
    },
    "favicon": "data:image/png;base64,<data>",
    "enforcesSecureChat": false
}"#;

fn main() -> anyhow::Result<()> {
    let con = TcpListener::bind("127.0.0.1:25565")?;

    for client in con.incoming() {
        println!("Accepted client!");
        let stream = client?;
        std::thread::spawn(move || {
            if let Err(e) = ClientHandler::new(stream).run() {
                println!("Error while handling client: {e}");
            }
        });
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum ServerState {
    Status,
    Login,
    Transfer,
    Handshaking,
}

struct ClientHandler {
    state: ServerState,
    stream: MyBufReader<MyBufWriter<TcpStream>>,
}

impl ClientHandler {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            state: ServerState::Handshaking,
            stream: MyBufReader::new(MyBufWriter::new(stream)),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            println!("\n=== {:?} ===", self.state);

            // Get packet length
            let packet_length = read_var_int(&mut self.stream, 3)?;
            println!("Packet length: {packet_length}");

            // Allocate & write to packet buffer
            let mut data_buf = vec![0u8; packet_length as usize];
            self.stream.read_exact(&mut data_buf)?;
            let data_reader = BufReader::new(data_buf.as_slice());

            // Hand off packet data to specified handler
            match self.state {
                ServerState::Status => self.process_status(data_reader),
                ServerState::Login => self.process_login(data_reader),
                ServerState::Transfer => self.process_status(data_reader),
                ServerState::Handshaking => self.process_handshaking(data_reader),
            }?;
        }
    }

    fn process_handshaking(&mut self, mut data: BufReader<&[u8]>) -> anyhow::Result<()> {
        let packet_id = read_var_int(&mut data, 5)?;
        println!("Packet ID: {packet_id}");

        match packet_id {
            0x00 => {
                // Read
                let protocol_version = read_var_int(&mut data, 5)?;
                println!("Protocol Version: {protocol_version}");

                let s = read_var_string(&mut data)?;
                println!("Server Address: {s}");

                let mut port = [0u8; 2];
                data.read_exact(&mut port)?;
                let port = u16::from_be_bytes(port);
                println!("Server Port: {port}");

                let intent = read_var_int(&mut data, 5)?;
                println!("Intent: {intent}");

                let new_state = match intent {
                    1 => ServerState::Status,
                    2 => ServerState::Login,
                    3 => ServerState::Transfer,
                    _ => bail!("Unknown status enum: {intent}"),
                };

                self.state = new_state;

                println!("Server state: {:?} -> {:?}", self.state, new_state);
            }
            _ => bail!("Unknown packet ID!"),
        }
        Ok(())
    }

    fn process_status(&mut self, mut data: BufReader<&[u8]>) -> anyhow::Result<()> {
        let packet_id = read_var_int(&mut data, 5)?;
        println!("Packet ID: {packet_id}");

        match packet_id {
            0x00 => {
                // Respond with status
                let mut response = Vec::new();
                response.write_all(&[0x00])?;
                write_var_string(&mut response, EXAMPLE_RESPONSE)?;
                write_response(&mut self.stream, &response)?;
                println!("Responded with status");
            }
            0x01 => {
                // Pong
                let mut buf = [0u8; 8];
                data.read_exact(&mut buf)?;
                let timestamp = i64::from_be_bytes(buf);
                println!("Timestamp from ping request: {timestamp}");

                let mut response = Vec::new();
                response.write_all(&[0x01])?;
                response.write_all(&buf)?;
                write_response(&mut self.stream, &response)?;
            }
            _ => bail!("Unknown packet ID!"),
        };

        Ok(())
    }

    fn process_login(&mut self, mut data: BufReader<&[u8]>) -> anyhow::Result<()> {
        todo!()
    }

    fn process_transfer(&mut self, mut data: BufReader<&[u8]>) -> anyhow::Result<()> {
        todo!()
    }
}
