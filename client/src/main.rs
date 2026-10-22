mod app;
mod crypto;
mod network;

use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;

fn main() -> eframe::Result<()> {
    let (commands_tx, commands_rx) = mpsc::channel(100);
    let (events_tx, events_rx) = std_mpsc::channel();

    let identity = crypto::Identity::generate();

    let registration = serde_json::json!({
        "type": "register",
        "user": "alice",
        "public_key": identity.public_hex(),
    })
    .to_string();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        runtime.block_on(async move {
            if let Err(error) =
                network::run("127.0.0.1:8080", commands_rx, events_tx, registration).await
            {
                eprintln!("Network error: {error}");
            }
        });
    });

    eframe::run_native(
        "Rust Messenger",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| {
            Ok(Box::new(app::MessengerApp {
                username: "alice".into(),
                recipient: "bob".into(),
                input: String::new(),
                messages: Vec::new(),
                commands: commands_tx,
                events: events_rx,
            }))
        }),
    )
}
