use std::{
    io::{BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
};

use anyhow::bail;
use socket2::Socket;

use crate::datacoding::*;
use crate::rwbuf::{MyBufReader, MyBufWriter};

const EXAMPLE_RESPONSE: &'static str = r#"{
    "version": {
        "name": "obServer",
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

pub struct MCProxy;

impl MCProxy {
    pub fn run(port: u16) -> anyhow::Result<()> {
        // create a TCP listener
        let socket = Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;

        let address: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let address = address.into();
        socket.set_reuse_address(true);
        socket.bind(&address)?;
        socket.listen(128)?;
        let listener: TcpListener = socket.into();

        for client in listener.incoming() {
            println!("Accepted client!");
            let stream = client?;
            std::thread::spawn(move || {
                if let Err(e) = ClientHandler::new(stream).run() {
                    println!("{e}");
                }
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum ServerState {
    Status,
    Login,
    Transfer,
    Handshaking,
}

pub struct ClientHandler {
    state: ServerState,
    exit: bool,
    stream: MyBufReader<MyBufWriter<TcpStream>>,
}

impl ClientHandler {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            state: ServerState::Handshaking,
            exit: false,
            stream: MyBufReader::new(MyBufWriter::new(stream)),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        while !self.exit {
            // Get packet length
            // We use the optional function here because it's the only place in the protocol that
            // expects an EOF. Everywhere else should treat a closed pipe as an error.
            let Some(packet_length) = read_var_int_optional(&mut self.stream, 3)? else {
                bail!("Client left!");
            };

            println!("\n=== {:?} ===", self.state);
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
        Ok(())
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

                let mut bytes = [0u8; 2];
                data.read_exact(&mut bytes)?;
                let port = u16::from_be_bytes(bytes);
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
            x => bail!("Unknown packet ID: {x}"),
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
                let mut bytes = [0u8; 8];
                data.read_exact(&mut bytes)?;
                let timestamp = i64::from_be_bytes(bytes);
                println!("Timestamp from ping request: {timestamp}");

                let mut response = Vec::new();
                response.write_all(&[0x01])?;
                response.write_all(&bytes)?;
                write_response(&mut self.stream, &response)?;
                println!("Ponged");
            }
            x => bail!("Unknown packet ID: {x}"),
        };

        Ok(())
    }

    fn process_login(&mut self, mut data: BufReader<&[u8]>) -> anyhow::Result<()> {
        let packet_id = read_var_int(&mut data, 5)?;
        println!("Packet ID: {packet_id}");

        match packet_id {
            0x00 => {
                let name = read_var_string(&mut data)?;
                let mut bytes = [0u8; 16];
                data.read_exact(&mut bytes)?;
                let uuid = u128::from_be_bytes(bytes);

                println!("Name: {name}");
                println!("UUID: {uuid}");

                let mut response = Vec::new();
                response.write_all(&[0x00])?;
                write_var_string(
                    &mut response,
                    r#""Server is starting up!\n Try logging back in after a minute.""#,
                )?;
                write_response(&mut self.stream, &response)?;
                self.exit = true;
            }
            x => bail!("Unknown packet ID: {x}"),
        };

        Ok(())
    }

    fn process_transfer(&mut self, mut data: BufReader<&[u8]>) -> anyhow::Result<()> {
        // Surely this works? I don't have a way to test this :(
        self.process_login(data)
    }
}
