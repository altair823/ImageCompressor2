use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use eframe::{epi, egui};
use egui::{Context, Slider};
use std::thread;
use std::sync::mpsc;
use image_compressor::folder_compress_with_channel;
use zip_compressor::compress_root_dir_to_7z;

use crate::epi::{Frame, Storage};

#[derive(Default)]
pub struct App{
    origin_dir: Arc<PathBuf>,
    dest_dir: Arc<PathBuf>,
    thread_count: u32,
    to_zip: bool,
    complete_file_list: Vec<String>,
    tr: Option<mpsc::Receiver<String>>,
    tx: Option<mpsc::Sender<String>>,
}

impl epi::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| ui.heading("Image Compress and Archive Program"));
            ui.separator();

            ui.heading("Original folder");
            if ui.button("select").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.origin_dir = Arc::new(path);
                }
                // self.complete_file_list.push(String::from(format!("{:?}", SystemTime::now()
                //     .duration_since(UNIX_EPOCH)
                //     .expect("Time went backwards"))));
            }
            let origin_dir = self.origin_dir.as_ref();
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut match origin_dir.to_str() {
                    Some(s) => s,
                    None => "",
                }).interactive(false)
                    .hint_text("Original folder"));
            });

            ui.separator();
            ui.heading("Destination folder");
            if ui.button("select").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.dest_dir = Arc::new(path);
                }
            }
            let dest_dir = self.dest_dir.as_ref();
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut match dest_dir.to_str(){
                    Some(s) => s,
                    None => "",
                }).interactive(false)
                    .hint_text("Destination folder"));
            });

            ui.separator();
            ui.heading("Thread count");
            ui.add(Slider::new(&mut self.thread_count, 1..=16).text("thread"));

            ui.separator();

            match &self.tr{
                Some(tr) => match tr.try_recv(){
                    Ok(s) => self.complete_file_list.push(s),
                    Err(_) => {}
                },
                None => {}
            }
            if ui.button("Compress!").clicked() {
                if !self.origin_dir.is_dir() || !self.dest_dir.is_dir() || self.thread_count <= 0{
                    return;
                }

                let origin = Arc::clone(&self.origin_dir);
                let dest = Arc::clone(&self.dest_dir);
                let tx = self.tx.clone();
                let th_count = self.thread_count;
                let z = self.to_zip;
                thread::spawn(move || {
                    match folder_compress_with_channel((origin.deref()).to_path_buf(),
                                                       (dest.deref()).to_path_buf(),
                                                       th_count,
                                                       tx.unwrap()) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Cannot compress the folder!: {}", e);
                        }
                    };
                    if z {
                        match compress_root_dir_to_7z(&(dest.deref()).to_path_buf(), &(dest.deref()).to_path_buf(), th_count) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Cannot archive the folder!: {}", e);
                            }
                        }
                    }
                });
            }
            ui.separator();
            ui.checkbox(&mut self.to_zip, "Archive with 7z");

            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);

                    let mut complete_files_string = String::new();

                    for line in self.complete_file_list.iter().rev(){
                        complete_files_string.push_str(&format!("{}\n", line));
                    }

                    ui.text_edit_multiline(&mut complete_files_string);
                });
            });


        });
    }

    fn setup(&mut self, _ctx: &Context, _frame: &Frame, _storage: Option<&dyn Storage>) {
        let (tx, tr) = mpsc::channel();
        self.tr = Some(tr);
        self.tx = Some(tx);
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
