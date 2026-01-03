#![allow(unused)]

use std::{
    io::{BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

use anyhow::bail;

use crate::rwbuf::{MyBufReader, MyBufWriter};
use crate::{
    datacoding::*,
    proxy::{ClientHandler, MCProxy},
};

mod datacoding;
mod proxy;
mod rwbuf;

fn main() -> anyhow::Result<()> {
    // If Java server is already running on specified port
    // Check every 5 minutes for active players

    let mut server_running = false;
    let online_players = 0;
    let port = 25565;

    loop {
        if server_running {
            std::thread::sleep(std::time::Duration::from_secs(300));
            if online_players == 0 {
                // TODO: shutdown server
                server_running = false;
            }
        } else {
            // Start the proxy and record its output (it should only return if it died or the
            // server needs to start up again)
            match MCProxy::run(port) {
                // Means server was asked to start up
                Ok(()) => {
                    // TODO: start server
                    server_running = true;
                }
                // Some critical error happened. Log it and exit
                Err(e) => {
                    bail!("Ran into fatal error when running the proxy: {e}");
                }
            }
        }
    }
}
