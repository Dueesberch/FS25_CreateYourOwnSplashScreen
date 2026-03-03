#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{Renderer, egui};
use std::path::PathBuf;
use sys_locale::get_locale;

mod converter;

struct I18n {
    is_german: bool,
}

impl I18n {
    fn new() -> Self {
        let locale = get_locale().unwrap_or_else(|| "en-US".to_string());
        Self {
            is_german: locale.starts_with("de"),
        }
    }

    fn t(&self, german: &str, english: &str) -> String {
        if self.is_german {
            german.to_string()
        } else {
            english.to_string()
        }
    }
}

struct ConverterApp {
    status: String,
    is_processing: bool,
    tx: std::sync::mpsc::Sender<String>,
    rx: std::sync::mpsc::Receiver<String>,
    last_generated_path: Option<PathBuf>,
    install_dir: Option<PathBuf>,
    i18n: I18n,
}

impl ConverterApp {
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            status: "Bereit.".to_string(),
            is_processing: false,
            tx,
            rx,
            last_generated_path: None,
            install_dir: converter::find_fs25_install_dir(),
            i18n: I18n::new(),
        }
    }
}

impl eframe::App for ConverterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(msg) = self.rx.try_recv() {
            if msg.starts_with("SUCCESS:") {
                let path_str = msg.replace("SUCCESS:", "");
                self.last_generated_path = Some(PathBuf::from(&path_str));
                self.status = format!("Konvertiert: {}", path_str);
            } else {
                self.status = msg;
            }
            self.is_processing = false;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading("DDS Converter 4K + Auto-Install");

                ui.horizontal(|ui| {
                    if let Some(path) = &self.install_dir {
                        let msg = self.i18n.t("FS25 gefunden", "FS25 found");
                        ui.colored_label(
                            egui::Color32::GREEN,
                            format!("✔ {}:\n{}", msg, path.display()),
                        );
                    } else {
                        ui.colored_label(
                            egui::Color32::RED,
                            self.i18n.t("✖ FS25 nicht gefunden", "✖ FS25 not found"),
                        );
                    }
                });

                ui.separator();

                ui.add_enabled_ui(!self.is_processing, |ui| {
                    if ui
                        .button(
                            self.i18n
                                .t("1. Bild wählen & konvertieren", "1. Select image & convert"),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(
                                "Bilder",
                                &["jpg", "jpeg", "png", "webp", "bmp", "tga", "tiff"],
                            )
                            .pick_file()
                        {
                            self.is_processing = true;
                            self.status = "Konvertiere...".to_string();
                            let tx_clone = self.tx.clone();
                            let ctx_clone = ctx.clone();

                            std::thread::spawn(move || {
                                match converter::convert_to_dds(path) {
                                    Ok(saved_path) => {
                                        let _ = tx_clone
                                            .send(format!("SUCCESS:{}", saved_path.display()));
                                    }
                                    Err(e) => {
                                        let _ = tx_clone.send(format!("Fehler: {}", e));
                                    }
                                }
                                ctx_clone.request_repaint();
                            });
                        }
                    }
                });

                ui.add_space(5.0);
                let can_install = self.last_generated_path.is_some() && self.install_dir.is_some();
                ui.add_enabled_ui(can_install && !self.is_processing, |ui| {
                    if ui
                        .button(self.i18n.t("2. In FS25 installieren", "2. Install to FS25"))
                        .clicked()
                    {
                        let source = self.last_generated_path.clone().unwrap();
                        let dest = self.install_dir.clone().unwrap();

                        match converter::install_to_game(source, dest) {
                            Ok(_) => {
                                self.status =
                                    "Erfolgreich installiert! (Backup erstellt)".to_string()
                            }
                            Err(e) => self.status = format!("Install-Fehler: {}", e),
                        }
                    }
                });

                ui.add_space(10.0);

                let status_msg = if self.status == "Bereit." {
                    self.i18n.t("Bereit.", "Ready.")
                } else if self.status == "Konvertiere..." {
                    self.i18n.t("Konvertiere...", "Converting...")
                } else {
                    self.status.clone()
                };

                ui.label(status_msg);
                if self.is_processing {
                    ui.spinner();
                }
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 200.0])
            .with_resizable(false),
        renderer: Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "FS25 Splash DDS Konverter",
        options,
        Box::new(|_cc| Ok(Box::new(ConverterApp::new()))),
    )
}
