mod file_io;

use std::borrow::Borrow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use eframe::{epi, egui};
use egui::{Context, Slider, TextEdit, Vec2};
use std::thread;
use std::sync::mpsc;
use image_compressor::FolderCompressor;
use zip_archive::{Archiver, get_dir_list_with_depth, Format};

use crate::epi::{Frame, Storage};
use crate::file_io::{ProgramData, DataType};

const ORIGIN_DIR_KEY: &str = "origin_dir";
const DESTINATION_DIR_KEY: &str = "destination_dir";
const ARCHIVE_DIR_KEY: &str = "archive_dir";
const TO_ZIP_KEY: &str = "to_zip";
const THREAD_COUNT_KEY: &str = "thread_count";
const DELETE_ORIGIN_KEY: &str = "delete_origin";
const ARCHIVE_FORMAT_KEY: &str = "archive_format";

pub const DEFAULT_SAVE_FILE_PATH: &str = "data/history.json";

#[derive(Default)]
pub struct App{
    program_data: ProgramData,
    origin_dir: Arc<Option<PathBuf>>,
    dest_dir: Arc<Option<PathBuf>>,
    archive_dir: Arc<Option<PathBuf>>,
    is_ui_enable: Arc<AtomicBool>,
    thread_count: u32,
    to_zip: bool,
    to_del_origin_files: bool,
    complete_file_list: Vec<String>,
    tr: Option<mpsc::Receiver<String>>,
    tx: Option<mpsc::Sender<String>>,
    archive_format: Format,
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

            let version = env!("CARGO_PKG_VERSION");

            // Title
            ui.vertical_centered(|ui| ui.heading(format!("Image Compress and Archive Program     v{}", version)));
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
                let origin_dir = match (*self.origin_dir).borrow() {
                    Some(p) => p.to_path_buf(),
                    None => PathBuf::new(),
                };
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut match origin_dir.to_str() {
                            Some(s) => s,
                            None => "",
                        }
                    ).interactive(false)
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
                let dest_dir = match (*self.dest_dir).borrow() {
                    Some(p) => p.to_path_buf(),
                    None => PathBuf::new(),
                };
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut match dest_dir.to_str() {
                            Some(s) => s,
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
                ui.checkbox(&mut self.to_zip, "Archive subdirectories");
                if self.to_zip {
                    ui.heading("Archive folder");
                    if ui.button("select").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.archive_dir = Arc::new(Some(path));
                        }
                    }
                    let archive_dir = match (*self.archive_dir).borrow() {
                        Some(p) => p.to_path_buf(),
                        None => PathBuf::new(),
                    };
                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut match archive_dir.to_str() {
                                Some(s) => s,
                                None => "",
                            }).interactive(false)
                            .hint_text("Archive folder"));
                    });
                    ui.label("Archive format: ");
                    ui.horizontal(|ui|{
                        ui.selectable_value(&mut self.archive_format, Format::Zip, "Zip");
                        ui.selectable_value(&mut self.archive_format, Format::Xz, "Xz");
                        ui.selectable_value(&mut self.archive_format, Format::_7z, "7z");
                    });
                }
                ui.separator();

                // Checkbox for deleting original files
                ui.checkbox(&mut self.to_del_origin_files, "Delete original files");
                ui.separator();

                // Compress button group
                ui.group(|ui| {

                    // Condition for compress
                    match &*(*self.origin_dir).borrow() {
                        Some(p) if !p.as_os_str().is_empty()  => {
                            match &*(*self.dest_dir).borrow() {
                                Some(p) if !p.as_os_str().is_empty() => {
                                    match self.to_zip {
                                        true => {
                                            match &*(*self.archive_dir).borrow() {
                                                Some(p) if !p.as_os_str().is_empty() => ui.set_enabled(true),
                                                _ => ui.set_enabled(false),
                                            }
                                        }
                                        false => ui.set_enabled(true),
                                    }
                                },
                            _ => ui.set_enabled(false),
                            }
                        },
                        _ => ui.set_enabled(false),
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
                        let to_del_origin = self.to_del_origin_files;
                        let origin_dir_list = get_dir_list_with_depth((*origin).as_ref().unwrap().to_path_buf(), 1).unwrap();
                        let archive_format = self.archive_format.clone();
                        
                        thread::spawn(move || {
                            let mut compressor = FolderCompressor::new((*origin).as_ref().unwrap().to_path_buf(), (*dest).as_ref().unwrap().to_path_buf());
                            compressor.set_thread_count(th_count);
                            compressor.set_delete_source(to_del_origin);
                            compressor.set_sender(compressor_tx.unwrap());
                            match compressor.compress() {
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
                                let mut archive_dir_list = Vec::new();
                                let dest_dir_list = get_dir_list_with_depth((*dest).as_ref().unwrap(), 1).unwrap();
                                for o_dir in origin_dir_list{
                                    for d_dir in &dest_dir_list{
                                        if o_dir.file_name().unwrap().eq(d_dir.file_name().unwrap()){
                                            archive_dir_list.push(d_dir.to_path_buf());
                                        }
                                    }
                                }
                                let mut archiver = Archiver::new();
                                archiver.set_destination((*archive).as_ref().unwrap().to_path_buf());
                                archiver.set_thread_count(th_count);
                                archiver.push_from_iter(archive_dir_list.iter());
                                archiver.set_sender(archive_tx.unwrap());
                                archiver.set_format(archive_format);
                                match archiver.archive() {
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
        let tx = self.tx.clone();
        self.program_data = match ProgramData::load(DEFAULT_SAVE_FILE_PATH){
            Ok(dir_set) => {
                if let Err(e) = tx.unwrap().send(String::from("Loading directory history complete!")) {
                    println!("Message passing error!: {}", e);
                }
                dir_set
            },
            Err(_) => {
                match tx.unwrap().send(String::from("Cannot load directory save file!\nSet save file path with default.")) {
                    Ok(_) => ProgramData::new(),
                    Err(e) => {
                        println!("Message passing error!: {}", e);
                        ProgramData::new()
                    },
                }
            }
        };

        self.origin_dir = match self.program_data.get_data(ORIGIN_DIR_KEY){
            Some(DataType::Directory(Some(p))) => Arc::new(Some(p.to_path_buf())),
            _ => Arc::new(Some(PathBuf::from(""))),
        };
        self.dest_dir = match self.program_data.get_data(DESTINATION_DIR_KEY) {
            Some(DataType::Directory(Some(p))) => Arc::new(Some(p.to_path_buf())),
            _ => Arc::new(Some(PathBuf::from(""))),
        };
        self.archive_dir = match self.program_data.get_data(ARCHIVE_DIR_KEY){
            Some(DataType::Directory(Some(p))) => Arc::new(Some(p.to_path_buf())),
            _ => Arc::new(Some(PathBuf::from(""))),
        };

        self.to_zip = match self.program_data.get_data(TO_ZIP_KEY) {
            Some(DataType::Boolean(Some(z))) => z.clone(),
            _ => false,
        };

        self.thread_count = match self.program_data.get_data(THREAD_COUNT_KEY) {
            Some(DataType::Number(Some(n))) => n.clone(),
            _ => 1,
        } as u32;

        self.to_del_origin_files = match self.program_data.get_data(DELETE_ORIGIN_KEY) {
            Some(DataType::Boolean(Some(b))) => b.clone(),
            _ => false,
        };

        self.archive_format = match self.program_data.get_data(ARCHIVE_FORMAT_KEY){
            Some(DataType::String(Some(b))) => Format::from(b),
            _ => Format::Zip,
        };
    }

    fn on_exit_event(&mut self) -> bool {
        self.program_data.set_data(ORIGIN_DIR_KEY, DataType::Directory(Some(match &(*self.origin_dir) {
            Some(p) => p.to_path_buf(),
            None => PathBuf::from(""),
        })));
        self.program_data.set_data(DESTINATION_DIR_KEY, DataType::Directory(Some(match &(*self.dest_dir) {
            Some(p) => p.to_path_buf(),
            None => PathBuf::from(""),
        })));
        self.program_data.set_data(ARCHIVE_DIR_KEY, DataType::Directory(Some(match &(*self.archive_dir) {
            Some(p) => p.to_path_buf(),
            None => PathBuf::from(""),
        })));
        self.program_data.set_data(TO_ZIP_KEY, DataType::Boolean(Some(self.to_zip)));
        self.program_data.set_data(THREAD_COUNT_KEY, DataType::Number(Some(self.thread_count as i32)));
        self.program_data.set_data(DELETE_ORIGIN_KEY, DataType::Boolean(Some(self.to_del_origin_files)));
        self.program_data.set_data(ARCHIVE_FORMAT_KEY, DataType::String(Some(self.archive_format.to_string())));

        match self.program_data.save(DEFAULT_SAVE_FILE_PATH){
            Ok(_) => {}
            Err(e) => println!("Cannot save the directory history! : {}", e),
        }
        return true;
    }

    fn name(&self) -> &str {
        "Image Compressor"
    }
}

