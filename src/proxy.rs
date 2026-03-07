// Structure of a Minecraft packet:
// -------------------------
// | Field     | Type      |
// -------------------------
// | Length    | VarInt    |
// | Packet ID | VarInt    |
// | Data      | Byte Arr  |
// -------------------------

use std::{
    io::{BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex, OnceLock, RwLock, mpsc},
};

use anyhow::{anyhow, bail};
use socket2::Socket;

use crate::rwbuf::{MyBufReader, MyBufWriter};
use protobuf::{VarRead, VarWrite};

const WIDTH: usize = 35;
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
    "favicon": "data:image/png;base64,<img-data>",
    "enforcesSecureChat": false
}"#;

static BLITTY_RESPONSE: OnceLock<String> = OnceLock::new();

#[derive(Debug)]
enum Event {
    ClientJoined(TcpStream),
    Shutdown,
}

/// Runs the Minecraft proxy server, spawning threads for each client that interacts with it
pub fn run_server(port: u16) -> anyhow::Result<()> {
    // create the blitty
    let blitty_str = EXAMPLE_RESPONSE.replace(
        "<img-data>",
        std::fs::read_to_string("./assets/blitty.b64")?.as_str(),
    );
    BLITTY_RESPONSE.set(blitty_str);

    // create a TCP listener
    let socket = Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;

    let address: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let address = address.into();
    socket.set_reuse_address(true);
    socket.bind(&address)?;
    socket.listen(128)?;
    let listener: TcpListener = socket.into();

    let (tx, rx) = mpsc::channel::<Event>();

    let listener_tx = tx.clone();
    // This thread will forever listen on the TcpListener and signal the main thread upon
    // connections
    std::thread::spawn(move || {
        for client in listener.incoming() {
            match client {
                Ok(stream) => listener_tx.send(Event::ClientJoined(stream)).unwrap(),
                Err(e) => println!("[ERROR] While attempting to connect to client: {e}"),
            }
        }
    });

    // Now just loop on incoming events so we know whether to create a new proxy session thread or
    // if we need to exit the loop and return control back to the main application
    loop {
        match rx.recv()? {
            Event::ClientJoined(tcp_stream) => {
                let thread_tx = tx.clone();
                std::thread::spawn(move || match ProxySession::new(&tcp_stream).run() {
                    Ok(_) => thread_tx.send(Event::Shutdown).unwrap(),
                    Err(e) => println!("[ERROR] While talking to client: {e}"),
                });
            }
            Event::Shutdown => return Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProxyState {
    Status,
    Login,
    Transfer,
    Handshaking,
}

pub struct ProxySession<'a> {
    state: ProxyState,
    exit: bool,
    reader: BufReader<&'a TcpStream>,
    writer: BufWriter<&'a TcpStream>,
    packet_buf: Vec<u8>,
}

impl<'a> ProxySession<'a> {
    pub fn new<'tcp: 'a>(stream: &'tcp TcpStream) -> Self {
        Self {
            state: ProxyState::Handshaking,
            reader: BufReader::new(&stream),
            writer: BufWriter::new(&stream),
            packet_buf: vec![],
            exit: false,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        while !self.exit {
            // Get packet length
            let packet_length: i32 = self
                .reader
                .read_var_int()
                .map_err(|_| anyhow!("Client left!"))?;

            println!("{:=^width$}", format!(" {:?} ", self.state), width = WIDTH);
            printkv("Packet length", packet_length);

            // Allocate & read full packet into buffer
            self.packet_buf.resize(packet_length as usize, 0x00);
            self.reader.read_exact(&mut self.packet_buf)?;

            let old_state = self.state;

            // Hand off packet data to specified handler
            match self.state {
                ProxyState::Status => self.process_status(),
                ProxyState::Login => self.process_login(),
                ProxyState::Transfer => self.process_status(),
                ProxyState::Handshaking => self.process_handshaking(),
            }?;

            println!("{:=<width$}\n", "", width = WIDTH);

            if old_state != self.state {
                println!("[STATE CHANGE] {:?} -> {:?}\n", old_state, self.state);
            }
        }

        println!("[PROXY] Closed TcpStream\n");
        Ok(())
    }

    fn process_handshaking(&mut self) -> anyhow::Result<()> {
        let mut data = self.packet_buf.as_slice();
        let packet_id = data.read_var_int()?;
        printkv("Packet ID", packet_id);
        match packet_id {
            0x00 => {
                let protocol_version: i32 = data.read_var_int()?;
                printkv("Protocol Version", protocol_version);

                let s = data.read_var_string()?;
                printkv("Server Address", s);

                let mut bytes = [0u8; 2];
                data.read_exact(&mut bytes)?;
                let port = u16::from_be_bytes(bytes);
                printkv("Server Port", port);

                let intent = data.read_var_int()?;
                printkv("Intent", intent);

                let new_state = match intent {
                    1 => ProxyState::Status,
                    2 => ProxyState::Login,
                    3 => ProxyState::Transfer,
                    _ => bail!("Unknown status enum: {intent}"),
                };

                self.state = new_state;
            }
            x => bail!("Unknown packet ID: {x}"),
        }
        Ok(())
    }

    fn process_status(&mut self) -> anyhow::Result<()> {
        let mut data = self.packet_buf.as_slice();
        let packet_id = data.read_var_int()?;
        printkv("Packet ID", packet_id);
        match packet_id {
            0x00 => {
                // Respond with status
                let mut response = Vec::new();
                response.write_all(&[0x00])?;
                response.write_var_string(BLITTY_RESPONSE.get().unwrap())?;
                self.writer.write_response(&response)?;
                println!("Responded with status");
            }
            0x01 => {
                // Pong
                let mut bytes = [0u8; 8];
                data.read_exact(&mut bytes)?;
                let timestamp = i64::from_be_bytes(bytes);
                printkv("Ping timestamp", timestamp);

                let mut response = Vec::new();
                response.write_all(&[0x01])?;
                response.write_all(&bytes)?;
                self.writer.write_response(&response)?;
            }
            x => bail!("Unknown packet ID: {x}"),
        };

        Ok(())
    }

    fn process_login(&mut self) -> anyhow::Result<()> {
        let mut data = self.packet_buf.as_slice();
        let packet_id = data.read_var_int()?;
        printkv("Packet ID", packet_id);
        match packet_id {
            0x00 => {
                let name = data.read_var_string()?;
                let mut bytes = [0u8; 16];
                data.read_exact(&mut bytes)?;
                let uuid = u128::from_be_bytes(bytes);

                printkv("Name", name);
                printkv("UUID", uuid);

                let mut response = Vec::new();
                response.write_all(&[0x00])?;
                response.write_var_string(
                    r#""Server is starting up!\n Try logging back in after a minute.""#,
                )?;
                self.writer.write_response(&response)?;
                self.exit = true;
            }
            x => bail!("Unknown packet ID: {x}"),
        };

        Ok(())
    }

    fn process_transfer(&mut self) -> anyhow::Result<()> {
        // Surely this works? I don't have a way to test this :(
        self.process_login()
    }
}

fn printkv(key: &str, value: impl std::fmt::Display) {
    println!("{:<20}{:>15}", format!("{key}:"), value);
}
