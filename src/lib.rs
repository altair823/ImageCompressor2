use std::path::PathBuf;
use eframe::{epi, egui};
use egui::Slider;
use image_compressor::folder_compress;

#[derive(Default)]
pub struct App{
    origin_dir: Option<PathBuf>,
    dest_dir: Option<PathBuf>,
    thread_count: i32,
}

impl epi::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Original folder");

            if let Some(picked_path) = &self.origin_dir {
                ui.horizontal(|ui| {
                    ui.label("Picked folder:");
                    ui.monospace(match picked_path.to_str(){
                        Some(s) => s,
                        None => "",
                    });
                });
            }
            if ui.button("select").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.origin_dir = Some(path);
                }
            }
            ui.heading("Destination folder");
            if let Some(dest_dir) = &self.dest_dir {
                ui.horizontal(|ui| {
                    ui.label("Picked folder:");
                    ui.monospace(match dest_dir.to_str(){
                        Some(s) => s,
                        None => "",
                    });
                });
            }
            if ui.button("select").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.dest_dir = Some(path);
                }
            }

            ui.heading("Thread count");
            ui.add(Slider::new(&mut self.thread_count, 1..=16).text("thread"));

            if ui.button("Compress!").clicked() {
                match folder_compress(&self.origin_dir.as_ref().unwrap(), &self.dest_dir.as_ref().unwrap(), self.thread_count) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Cannot compress the folder!: {}", e);
                    }
                };
            }
        });
    }

    fn name(&self) -> &str {
        "My egui App"
    }
}

#[cfg(test)]
mod tests {
    use crate::App;

    #[test]
    fn it_works() {
        let app = App::default();
        let native_options = eframe::NativeOptions::default();
        eframe::run_native(Box::new(app), native_options);
    }
}
