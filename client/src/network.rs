use std::io;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};

pub enum Command {
    Send(String),
}

pub enum Event {
    Received(String),
    Disconnected,
}

pub async fn run(
    addr: &str,
    mut commands: mpsc::Receiver<Command>,
    events: std::sync::mpsc::Sender<Event>,
    registration: String,
) -> io::Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let (reader, mut writer) = stream.into_split();

    writer.write_all(registration.as_bytes()).await?;
    writer.write_all(b"\n").await?;

    let mut lines = BufReader::new(reader).lines();

    loop {
        tokio::select! {
            result = lines.next_line() => {
                match result? {
                    Some(line) => {
                        if events.send(Event::Received(line)).is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }

            command = commands.recv() => {
                match command {
                    Some(Command::Send(data)) => {
                        writer.write_all(data.as_bytes()).await?;
                        writer.write_all(b"\n").await?;
                    }
                    None => break,
                }
            }
        }
    }

    let _ = events.send(Event::Disconnected);
    Ok(())
}
