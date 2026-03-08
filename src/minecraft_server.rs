use std::process::{Child, Command, Stdio};

use async_std::{net::ToSocketAddrs, task};
use rcon::{AsyncStdStream, Connection};

pub struct MinecraftServer {
    pub rcon: Option<Connection<AsyncStdStream>>,
    pub child: Option<Child>,
    address: String,
    password: String,
}

impl MinecraftServer {
    pub fn new(address: &str, password: &str) -> Self {
        Self {
            rcon: None,
            child: None,
            address: address.to_string(),
            password: password.to_string(),
        }
    }

    pub fn start(&mut self, cmd: &str, args: &[&str]) -> anyhow::Result<()> {
        self.child = Some(
            Command::new(cmd)
                .args(args)
                .stdin(Stdio::null())
                .spawn()?,
        );
        Ok(())
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(conn) = self.rcon.as_mut() {
            let _ = conn.cmd("stop").await;
        }
        if let Some(mut child) = self.child.take() {
            task::spawn_blocking(move || child.wait()).await?;
        }
        self.rcon = None;
        Ok(())
    }

    /// Checks player count. If RCON fails, it clears the connection and returns an error.
    pub async fn get_player_count(&mut self) -> Result<usize, MinecraftError> {
        let conn = self.rcon.as_mut().ok_or(MinecraftError::Uninitialized)?;

        match get_players(conn).await {
            Ok(count) => Ok(count),
            Err(e) => {
                self.rcon = None; // Connection died
                Err(e)
            }
        }
    }

    /// Connect to the server (via RCON).
    pub async fn connect(&mut self, address: &str, password: &str) -> anyhow::Result<()> {
        let mut conn = Connection::connect(address, password).await?;
        let query = get_players(&mut conn).await;
        self.rcon = Some(conn);
        self.address = address.to_string();
        self.password = password.to_string();
        Ok(())
    }

    /// Ping the server (by checking if its RCON is interactable). Upon a success, returns Ok(()),
    /// and the struct's fields will be updated.
    pub async fn ping(&mut self) -> Result<(), MinecraftError> {
        let Some(conn) = self.rcon.as_mut() else {
            return Err(MinecraftError::Uninitialized)?;
        };

        let query = get_players(conn).await;
        Ok(())
    }
}

async fn get_players(conn: &mut Connection<AsyncStdStream>) -> Result<usize, MinecraftError> {
    let output = conn
        .cmd("list")
        .await
        .map_err(|_| MinecraftError::ConnectionClosed)?;
    output
        .split(' ')
        .find_map(|s| s.parse::<usize>().ok())
        .ok_or(MinecraftError::BadRconOutput)
}

#[derive(Debug, Clone, Copy)]
pub enum MinecraftError {
    ConnectionClosed,
    Uninitialized,
    BadRconOutput,
}

impl std::fmt::Display for MinecraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self}"))
    }
}

impl std::error::Error for MinecraftError {}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::minecraft_server::MinecraftServer;
}
