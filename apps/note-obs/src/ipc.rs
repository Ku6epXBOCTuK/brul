//! Один экземпляр: свободный UDP-порт как замок, команды — дейтаграммы.

use std::io;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use brul::Handle;
use brul::logging::{error, info, warn};

pub const SHOW: &str = "show";
pub const QUIT: &str = "quit";

const ADDRESS: &str = "127.0.0.1:44571";
const BUFFER: usize = 64;
const TIMEOUT: Duration = Duration::from_millis(500);

pub fn bind() -> io::Result<UdpSocket> {
    bind_at(ADDRESS)
}

pub fn send(command: &str) -> io::Result<()> {
    send_to(command, ADDRESS)
}

pub fn serve(socket: UdpSocket, handle: Handle) -> io::Result<()> {
    thread::Builder::new()
        .name("nobs-ipc".to_owned())
        .spawn(move || {
            let mut buffer = [0; BUFFER];
            loop {
                let size = match socket.recv(&mut buffer) {
                    Ok(size) => size,
                    Err(failure)
                        if matches!(
                            failure.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(failure) => {
                        error!(%failure, "ipc receive failed");
                        return;
                    }
                };
                let Ok(command) = std::str::from_utf8(&buffer[..size]) else {
                    warn!("ipc command is not utf-8");
                    continue;
                };
                match command.trim() {
                    SHOW => handle.call("trayShow"),
                    QUIT => handle.call("trayQuit"),
                    other => info!(command = other, "unknown ipc command"),
                }
            }
        })?;
    Ok(())
}

fn bind_at(address: &str) -> io::Result<UdpSocket> {
    let socket = UdpSocket::bind(address)?;
    socket.set_read_timeout(Some(TIMEOUT))?;
    Ok(socket)
}

fn send_to(command: &str, address: &str) -> io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    socket.send_to(command.as_bytes(), address)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SHOW, bind_at, send_to};

    #[test]
    fn the_lock_port_is_taken_only_once() {
        let first = bind_at("127.0.0.1:47111").expect("first instance");
        assert!(bind_at("127.0.0.1:47111").is_err());
        drop(first);
        assert!(bind_at("127.0.0.1:47111").is_ok());
    }

    #[test]
    fn a_command_reaches_the_listener() {
        let socket = bind_at("127.0.0.1:47112").expect("listener");
        send_to(SHOW, "127.0.0.1:47112").expect("send");

        let mut buffer = [0; 64];
        let size = socket.recv(&mut buffer).expect("receive");

        assert_eq!(&buffer[..size], SHOW.as_bytes());
    }
}
