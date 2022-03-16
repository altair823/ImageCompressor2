use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use crossbeam::atomic::AtomicCell;
use eframe::{epi, egui};
use egui::Slider;
use image_compressor::folder_compress;

#[derive(Default)]
pub struct App{
    origin_dir: Arc<Option<PathBuf>>,
    dest_dir: Arc<Option<PathBuf>>,
    thread_count: i32,
}

impl epi::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Original folder");

            if let Some(picked_path) = &self.origin_dir.as_ref() {
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
                    self.origin_dir = Arc::new(Some(path));
                }
            }
            ui.heading("Destination folder");
            if let Some(dest_dir) = &self.dest_dir.as_ref() {
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
                    self.dest_dir = Arc::new(Some(path));
                }
            }

            ui.heading("Thread count");
            ui.add(Slider::new(&mut self.thread_count, 1..=16).text("thread"));

            if ui.button("Compress!").clicked() {
                let s = self.origin_dir.clone();
                let d = self.dest_dir.clone();
                thread::spawn(move ||{
                    let o = s;
                    match folder_compress(&o.unwrap(), &d.unwrap(), 5){
                        Ok(_) => {}
                        Err(e) => {
                            println!("Cannot compress the folder!: {}", e);
                        }
                    }
                });
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
