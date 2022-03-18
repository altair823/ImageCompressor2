use eframe::{NativeOptions, run_native};
use egui::Vec2;
use ImageCompressor::App;


fn main() {
    #![windows_subsystem = "windows"]
    let app = App::default();
    let mut win_option = NativeOptions::default();
    win_option.initial_window_size = Some(Vec2::new(480., 795.));
    win_option.min_window_size = Some(Vec2::new(480., 795.));
    win_option.resizable = false;
    run_native(Box::new(app), win_option);
}
