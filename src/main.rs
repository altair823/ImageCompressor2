use eframe;
use eframe::{NativeOptions, run_native};
use eframe::epaint::Shape::Vec;
use egui::Vec2;
use ImageCompressor::App;

fn main() {
    let app = App::default();
    let mut win_option = NativeOptions::default();
    win_option.initial_window_size = Some(Vec2::new(480., 240.));
    run_native(Box::new(app), win_option);
}
