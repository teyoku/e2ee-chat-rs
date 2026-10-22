use std::{collections::HashMap, io, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{RwLock, mpsc},
};

#[derive(Clone)]
struct Client {
    public_key: String,
    tx: mpsc::Sender<Packet>,
}

type Clients = Arc<RwLock<HashMap<String, Client>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Packet {
    Register {
        user: String,
        public_key: String,
    },
    GetKey {
        user: String,
    },
    PublicKey {
        user: String,
        key: String,
    },
    Message {
        from: String,
        to: String,
        nonce: String,
        ciphertext: String,
    },
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let clients = Arc::new(RwLock::new(HashMap::new()));

    println!("Server listening on 127.0.0.1:8080");

    loop {
        let (stream, _) = listener.accept().await?;
        let clients = clients.clone();

        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, clients).await {
                eprintln!("Client error: {err}");
            }
        });
    }
}

async fn handle_client(stream: TcpStream, clients: Clients) -> io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    let (tx, mut rx) = mpsc::channel::<Packet>(100);

    let Some(first_line) = lines.next_line().await? else {
        return Ok(());
    };

    let Ok(Packet::Register { user, public_key }) = serde_json::from_str::<Packet>(&first_line)
    else {
        return Ok(());
    };

    if user.is_empty() || user.len() > 64 {
        return Ok(());
    }

    {
        let mut peers = clients.write().await;

        if peers.contains_key(&user) {
            return Ok(());
        }

        peers.insert(
            user.clone(),
            Client {
                public_key,
                tx: tx.clone(),
            },
        );
    }

    println!("{user} connected");

    let write_task = tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            let Ok(data) = serde_json::to_string(&packet) else {
                break;
            };

            if writer.write_all(data.as_bytes()).await.is_err()
                || writer.write_all(b"\n").await.is_err()
            {
                break;
            }
        }
    });

    while let Some(line) = lines.next_line().await? {
        if line.len() > 64 * 1024 {
            break;
        }

        let Ok(packet) = serde_json::from_str::<Packet>(&line) else {
            continue;
        };

        match packet {
            Packet::GetKey { user: target } => {
                let peer = clients.read().await.get(&target).cloned();

                if let Some(peer) = peer {
                    let _ = tx
                        .send(Packet::PublicKey {
                            user: target,
                            key: peer.public_key,
                        })
                        .await;
                }
            }
            Packet::Message {
                to,
                nonce,
                ciphertext,
                ..
            } => {
                let peer = clients.read().await.get(&to).cloned();

                if let Some(peer) = peer {
                    let _ = peer
                        .tx
                        .send(Packet::Message {
                            from: user.clone(),
                            to,
                            nonce,
                            ciphertext,
                        })
                        .await;
                }
            }
            _ => {}
        }
    }

    clients.write().await.remove(&user);
    write_task.abort();

    println!("{user} disconnected");

    Ok(())
}
