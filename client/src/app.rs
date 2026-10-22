use eframe::egui;
use tokio::sync::mpsc;

use crate::network::{Command, Event};

pub struct MessengerApp {
    pub username: String,
    pub recipient: String,
    pub input: String,
    pub messages: Vec<String>,

    pub commands: mpsc::Sender<Command>,
    pub events: std::sync::mpsc::Receiver<Event>,
}

impl eframe::App for MessengerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Received(packet) => {
                    self.messages.push(packet);
                }
                Event::Disconnected => {
                    self.messages.push("Connection lost".into());
                }
            }
        }

        ui.heading("Rust Messenger");
        ui.horizontal(|ui| {
            ui.label(format!("You: {}", self.username));
            ui.separator();
            ui.label("Receiver:");
            ui.text_edit_singleline(&mut self.recipient);
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for message in &self.messages {
                    ui.label(message);
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.input).desired_width(ui.available_width()));

            if ui.button("Send").clicked() && !self.input.is_empty() {
                let plaintext = std::mem::take(&mut self.input);

                let _ = plaintext;
            }
        });

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }
}
