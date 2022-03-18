use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use eframe::{epi, egui};
use egui::{Context, Slider, TextEdit, Vec2};
use std::thread;
use std::sync::mpsc;
use image_compressor::folder_compress_with_channel;
use zip_archive::archive_root_dir_with_sender;

use crate::epi::{Frame, Storage};

#[derive(Default)]
pub struct App{
    is_ui_enable: Arc<AtomicBool>,
    origin_dir: Arc<Option<PathBuf>>,
    dest_dir: Arc<Option<PathBuf>>,
    archive_dir: Arc<Option<PathBuf>>,
    thread_count: u32,
    to_zip: bool,
    complete_file_list: Vec<String>,
    tr: Option<mpsc::Receiver<String>>,
    tx: Option<mpsc::Sender<String>>,
}

impl epi::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            match &self.tr {
                Some(tr) => match tr.try_recv() {
                    Ok(s) => self.complete_file_list.push(s),
                    Err(_) => {}
                },
                None => {}
            }

            // Title
            ui.vertical_centered(|ui| ui.heading("Image Compress and Archive Program"));
            ui.add_space(10.);

            // UI group
            ui.group(|ui| {
                ui.set_enabled((*self.is_ui_enable).load(Ordering::Relaxed));

                // Original folder selector
                ui.heading("Original folder");
                if ui.button("select").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.origin_dir = Arc::new(Some(path));
                    }
                }
                let origin_dir = self.origin_dir.as_ref();
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut match origin_dir.as_ref() {
                        Some(s) => match s.to_str() {
                            Some(s) => s,
                            None => "",
                        },
                        None => "",
                    }).interactive(false)
                        .hint_text("Original folder"));
                });
                ui.separator();

                // Destination folder selector
                ui.heading("Destination folder");
                if ui.button("select").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.dest_dir = Arc::new(Some(path));
                    }
                }
                let dest_dir = self.dest_dir.as_ref();
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut match dest_dir.as_ref() {
                        Some(s) => match s.to_str() {
                            Some(s) => s,
                            None => "",
                        },
                        None => "",
                    }).interactive(false)
                        .hint_text("Destination folder"));
                });
                ui.separator();

                // Thread count slider
                ui.heading("Thread count");
                ui.add(Slider::new(&mut self.thread_count, 1..=16).text("thread"));
                ui.separator();

                // Checkbox for archiving
                // Archiving folder selector
                ui.checkbox(&mut self.to_zip, "Archive with 7z");
                if self.to_zip {
                    ui.heading("Archive folder");
                    if ui.button("select").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.archive_dir = Arc::new(Some(path));
                        }
                    }
                    let archive_dir = self.archive_dir.as_ref();
                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut match archive_dir.as_ref() {
                            Some(s) => match s.to_str() {
                                Some(s) => s,
                                None => "",
                            },
                            None => "",
                        }).interactive(false)
                            .hint_text("Archive folder"));
                    });
                }
                ui.separator();

                // Compress button group
                ui.group(|ui| {

                    // Condition for compress
                    match *self.origin_dir {
                        None => ui.set_enabled(false),
                        Some(_) => {
                            match *self.dest_dir {
                                None => ui.set_enabled(false),
                                Some(_) => {
                                    match self.to_zip {
                                        true => {
                                            match *self.archive_dir {
                                                None => ui.set_enabled(false),
                                                Some(_) => ui.set_enabled(true),
                                            }
                                        }
                                        false => ui.set_enabled(true),
                                    }
                                },
                            }
                        },
                    }

                    // Compress button
                    let compress_button = egui::Button::new("Compress");
                    if ui.add_sized(Vec2::new(ui.available_width(), 40.), compress_button).clicked() {
                        self.is_ui_enable.swap(false, Ordering::Relaxed);
                        let origin = Arc::clone(&self.origin_dir);
                        let dest = Arc::clone(&self.dest_dir);
                        let archive = Arc::clone(&self.archive_dir);
                        let is_ui_enable = Arc::clone(&self.is_ui_enable);
                        let compressor_tx = self.tx.clone();
                        let archive_tx = self.tx.clone();
                        let th_count = self.thread_count;
                        let z = self.to_zip;
                        thread::spawn(move || {
                            match folder_compress_with_channel((*origin).as_ref().unwrap().to_path_buf(),
                                                               (*dest).as_ref().unwrap().to_path_buf(),
                                                               th_count,
                                                               compressor_tx.unwrap()) {
                                Ok(_) => {
                                    if !z {
                                        is_ui_enable.swap(true, Ordering::Relaxed);
                                    }
                                },
                                Err(e) => {
                                    println!("Cannot compress the folder!: {}", e);
                                }
                            };
                            if z {
                                match archive_root_dir_with_sender(&(*dest).as_ref().unwrap().to_path_buf(),
                                                                   &(*archive).as_ref().unwrap().to_path_buf(),
                                                                   th_count,
                                                                   archive_tx.unwrap()) {
                                    Ok(_) => { is_ui_enable.swap(true, Ordering::Relaxed); }
                                    Err(e) => {
                                        println!("Cannot archive the folder!: {}", e);
                                    }
                                }
                            }
                        });
                    }
                });
            });
            ui.add_space(10.);

            // TextEdit for status dialog
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);

                    let mut complete_files_string = String::new();

                    for line in self.complete_file_list.iter().rev(){
                        complete_files_string.push_str(&format!("{}\n", line));
                    }

                    let status_dialog = TextEdit::multiline(&mut complete_files_string).interactive(false).desired_rows(25);
                    ui.add_sized(ui.available_size(), status_dialog);
                    frame.request_repaint();
                });
            });
        });
    }

    fn setup(&mut self, _ctx: &Context, _frame: &Frame, _storage: Option<&dyn Storage>) {
        let (tx, tr) = mpsc::channel();
        self.tr = Some(tr);
        self.tx = Some(tx);
        self.thread_count = 1;
        self.is_ui_enable = Arc::new(AtomicBool::new(true));
    }

    fn name(&self) -> &str {
        "Image Compressor"
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
