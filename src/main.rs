#![allow(unused)]

use std::{
    io::{BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
};

use anyhow::bail;
use async_std::task;
use rcon::{AsyncStdStream, Connection};

use crate::{minecraft_server::MinecraftServer, proxy::ProxyError};

mod minecraft_server;
mod proxy;

fn main() -> anyhow::Result<()> {
    task::block_on(async {
        // If Java server is already running on specified port
        // Check every 5 minutes for active players

        let mut state = ServerState::Stopped;
        let port = 25565;
        let mut mc_server = MinecraftServer::new("127.0.0.1:25575", "bobby");

        let start_command: &str = "sh";
        let start_command_args: &[&str] =
            &["-c", "podman compose -f ./minecraft/podman-compose.yml up"];

        loop {
            match state {
                ServerState::Running => {
                    task::sleep(std::time::Duration::from_secs(30)).await;

                    match mc_server.get_player_count().await {
                        Ok(0) => {
                            println!("0 players. Shutting down...");
                            mc_server.stop().await?;
                            state = ServerState::Stopped;
                        }

                        Ok(n) => println!("{n} players online. Staying awake."),

                        // Server is offline. Maybe it crashed? whatever the case, let it restart
                        // on its own via player-join
                        Err(_) => {
                            println!("Lost RCON connection. Assuming server crashed.");
                            state = ServerState::Stopped;
                        }
                    }
                }
                ServerState::Stopped => {
                    // Start the proxy and record its output (it should only return if it errored or the
                    // server needs to start up again)
                    match proxy::run_server(port) {
                        // Means server was asked to start up
                        Ok(()) => {
                            println!("Starting server!");
                            state = ServerState::ShouldStart;
                        }
                        // Means server could not connect on the address. Could be because the Java
                        // process was connected to the socket already. In case it's on cooldown,
                        // restart after a few seconds
                        Err(e) if e == ProxyError::FailedToBind => {
                            println!("Failed to bind to given address/port, waiting 5 seconds...");
                            task::sleep(std::time::Duration::from_secs(5)).await;
                            println!("Retrying now...");
                        }
                        // Some unrecoverable error happened. Log it and exit
                        Err(e) => {
                            bail!("Ran into fatal error when running the proxy: {e}");
                        }
                    }
                }
                ServerState::ShouldStart => {
                    task::sleep(std::time::Duration::from_secs(5)).await;
                    println!("Running start command!");
                    mc_server.start(start_command, start_command_args)?;
                    state = ServerState::Starting;
                }
                ServerState::Starting => {
                    task::sleep(std::time::Duration::from_secs(10)).await;
                    // If connect succeeds, we know the server is ready
                    if mc_server
                        .connect("127.0.0.1:25575", "password")
                        .await
                        .is_ok()
                    {
                        println!("Server is up!");
                        state = ServerState::Running;
                    }
                }
            }
        }
    })
}

#[derive(Debug, Clone, Copy)]
enum ServerState {
    Running,
    Stopped,
    ShouldStart,
    Starting,
}
