//! # Image compressor
//!
//! `image_compressor` is a library that compresses images with multiple threads.
//! See [image](https://crates.io/crates/image) crate for check the extention that supported.
//!
//! If you want to compress a single image, see [`Compressor`](compressor::Compressor) struct.
//! 
//! Or if you want to compress multiple images in a certain directory, see [`FolderCompressor`] struct. 
//! It compresses images by using multithread. 
//! 
//! To use these structs and its functions, you need to give them a function pointer or closure 
//! that calculate size and quality of new compressed images. 
//! That calculator function(or closure) need to calculate and returns a [`Factor`] 
//! base on image size and file size of the original image. 
//! To see more information about it, see [`Factor`]. 
//! 
//! # Examples
//! 
//! `FolderCompressor` and its `compress` function example.
//!
//! The function compress all images in given origin folder with multithread at the same time,
//! and wait until everything is done. This function does not using any `mpsc::Sender`.
//! ```
//! use std::path::PathBuf;
//! use std::sync::mpsc;
//! use image_compressor::FolderCompressor;
//! use image_compressor::Factor;
//!
//! let origin = PathBuf::from("origin_dir");   // original directory path
//! let dest = PathBuf::from("dest_dir");       // destination directory path
//! let thread_count = 4;                       // number of threads
//! 
//! let mut comp = FolderCompressor::new(origin, dest);
//! comp.set_cal_func(|width, height, file_size| {return Factor::new(75., 0.7)});
//! comp.set_thread_count(4);
//! match comp.compress(){
//!     Ok(_) => {},
//!     Err(e) => println!("Cannot compress the folder!: {}", e),
//! }
//! ```
//! 
//! `FolderCompressor` and its `compress_with_sender` example.
//!
//! The function compress all images in given origin folder with multithread at the same time,
//! and wait until everything is done. 
//! 
//! With `mpsc::Sender` (argument `tx` in this example),
//! the process running in this function will dispatch a message indicating whether image compression is complete.
//! ```
//! use std::path::PathBuf;
//! use std::sync::mpsc;
//! use image_compressor::FolderCompressor;
//! use image_compressor::Factor;
//!
//! let origin = PathBuf::from("origin_dir");       // original directory path
//! let dest = PathBuf::from("dest_dir");           // destination directory path
//! let thread_count = 4;                           // number of threads
//! let (tx, tr) = mpsc::channel();                 // Sender and Receiver. for more info, check mpsc and message passing. 
//! 
//! let mut comp = FolderCompressor::new(origin, dest);
//! comp.set_cal_func(|width, height, file_size| {return Factor::new(75., 0.7)});
//! comp.set_thread_count(4);
//! match comp.compress_with_sender(tx.clone()) {
//!     Ok(_) => {},
//!     Err(e) => println!("Cannot compress the folder!: {}", e),
//! }
//! ```
//!
//! `Compressor` and `compress_to_jpg` example.
//! 
//! Compressing just a one image. 
//! ```
//! use std::path::PathBuf;
//! use image_compressor::compressor::Compressor;
//! use image_compressor::Factor;
//!
//! let origin_dir = PathBuf::from("origin").join("file1.jpg");
//! let dest_dir = PathBuf::from("dest");
//!
//! let comp = Compressor::new(origin_dir, dest_dir, |width, height, file_size| {return Factor::new(75., 0.7)});
//! comp.compress_to_jpg();
//! ```

use std::error::Error;
use std::fs;
use std::path::{PathBuf, Path};
use std::sync::{Arc, mpsc};
use compressor::Compressor;
use crawler::get_file_list;
use std::thread;
use crossbeam_queue::SegQueue;

pub mod crawler;
pub mod compressor;

pub use compressor::Factor;

fn default_cal_func(_width: u32, _height: u32, file_size: u64) -> Factor {
    return match file_size{
        file_size if file_size > 5000000 => Factor::new(60., 0.7),
        file_size if file_size > 1000000 => Factor::new(65., 0.75),
        file_size if file_size > 500000 => Factor::new(70., 0.8),
        file_size if file_size > 300000 => Factor::new(75., 0.85),
        file_size if file_size > 100000 => Factor::new(80., 0.9),
        _ => Factor::new(85., 1.0),
    }
}

/// Compressor struct for a directory.
pub struct FolderCompressor{
    calculate_quality_and_size: Arc<fn(u32, u32, u64) -> Factor>,
    original_path: PathBuf,
    destination_path: PathBuf,
    thread_count: u32,
}

impl FolderCompressor {

    /// Create a new `FolderCompressor` instance.
    /// Just needs original directory path and destination directory path. 
    /// If you do not set the quality calculation function, 
    /// it will use the default calculation function which sets the quality only by the file size. 
    /// Likewise, if you do not set the number of threads, only one thread is used by default.\
    /// # Examples
    /// ```
    /// use image_compressor::FolderCompressor;
    /// use std::path::Path;
    /// 
    /// let origin = Path::new("origin");
    /// let dest = Path::new("dest");
    /// 
    /// let comp = FolderCompressor::new(origin, dest);
    /// ```
    pub fn new<O: AsRef<Path>, D: AsRef<Path>>(origin_path: O, dest_path: D) -> Self{
        FolderCompressor { 
            calculate_quality_and_size: Arc::new(default_cal_func), 
            original_path: origin_path.as_ref().to_path_buf(), 
            destination_path: dest_path.as_ref().to_path_buf(), 
            thread_count: 1 }
    }

    /// Setter for calculation function that return a Factor using to compress images. 
    /// 
    /// # Examples
    /// ```
    /// use image_compressor::FolderCompressor;
    /// use image_compressor::Factor;
    /// use std::path::Path;
    /// 
    /// let origin = Path::new("origin");
    /// let dest = Path::new("dest");
    /// 
    /// let mut comp = FolderCompressor::new(origin, dest);
    /// comp.set_cal_func(|width, height, file_size| {return Factor::new(75., 0.7)});
    /// ```
    pub fn set_cal_func(&mut self, cal_func: fn(u32, u32, u64) -> Factor){
        self.calculate_quality_and_size = Arc::new(cal_func);
    }

    /// Setter for the number of threads used to compress images. 
    /// # Examples
    /// ```
    /// use image_compressor::FolderCompressor;
    /// use image_compressor::Factor;
    /// use std::path::Path;
    /// 
    /// let origin = Path::new("origin");
    /// let dest = Path::new("dest");
    /// 
    /// let mut comp = FolderCompressor::new(origin, dest);
    /// comp.set_thread_count(4);
    /// ```
    pub fn set_thread_count(&mut self, thread_count: u32){
        self.thread_count = thread_count;
    }

    /// Folder compress function with mpsc::Sender.
    ///
    /// The function compress all images in given origin folder with multithread at the same time,
    /// and wait until everything is done. With `mpsc::Sender` (argument `tx` in this example),
    /// the process running in this function will dispatch a message indicating whether image compression is complete.
    /// 
    /// # Warning
    /// Since this function comsume its `self`, the `FolderCompressor` instance (which is self) is no longer available after calling this function. 
    /// ```
    /// use std::path::PathBuf;
    /// use std::sync::mpsc;
    /// use image_compressor::FolderCompressor;
    ///
    /// let origin = PathBuf::from("origin_dir");
    /// let dest = PathBuf::from("dest_dir");
    /// let (tx, tr) = mpsc::channel();
    /// 
    /// let mut comp = FolderCompressor::new(origin, dest);
    /// comp.set_thread_count(4);
    /// 
    /// match folder_compressor.compress_with_sender(tx.clone()) {
    ///     Ok(_) => {},
    ///     Err(e) => println!("Cannot compress the folder!: {}", e),
    /// }
    /// ```
    pub fn compress_with_sender(
        self,
        sender: mpsc::Sender<String>) -> Result<(), Box<dyn Error>> {
        let to_comp_file_list = get_file_list(&self.original_path)?;
        match sender.send(format!("Total file count: {}", to_comp_file_list.len())) {
            Ok(_) => {},
            Err(e) => {
                println!("Message passing error!: {}", e);
            }
        }

        let queue = Arc::new(SegQueue::new());
        for i in to_comp_file_list{
            queue.push(i);
        }
        let mut handles = Vec::new();
        let arc_root = Arc::new(self.original_path);
        let arc_dest = Arc::new(self.destination_path);
        for _ in 0..self.thread_count {
            let new_sender = sender.clone();
            let arc_root = Arc::clone(&arc_root);
            let arc_dest = Arc::clone(&arc_dest);
            let arc_queue = Arc::clone(&queue);
            let arc_cal_func = Arc::clone(&self.calculate_quality_and_size);
            let handle = thread::spawn(move || {
                process_with_sender(arc_queue, &arc_root, &arc_dest, *arc_cal_func, new_sender);
            });
            handles.push(handle);
        }

        for h in handles{
            h.join().unwrap();
        }
        match sender.send(String::from("Compress complete!")){
            Ok(_) => {},
            Err(e) => {
                println!("Message passing error!: {}", e);
            }
        };
        // let new_sender = mpsc::Sender::clone(&sender);
        // process_with_sender(&queue, &dest, &root, new_sender);
        return Ok(());
    }

    /// Folder compress function.
    ///
    /// The function compress all images in given origin folder with multithread at the same time,
    /// and wait until everything is done. This function does not send any messages.
    /// 
    /// # Warning
    /// Since this function comsume its `self`, the `FolderCompressor` instance (which is self) is no longer available after calling this function. 
    /// ```
    /// use std::path::PathBuf;
    /// use std::sync::mpsc;
    /// use image_compressor::FolderCompressor;
    ///
    /// let origin = PathBuf::from("origin_dir");
    /// let dest = PathBuf::from("dest_dir");
    /// 
    /// let mut comp = FolderCompressor::new(origin, dest);
    /// comp.set_thread_count(4);
    /// 
    /// match folder_compressor.compress(){
    ///     Ok(_) => {},
    ///     Err(e) => println!("Cannot compress the folder!: {}", e),
    /// }
    /// ```
    pub fn compress(self) -> Result<(), Box<dyn Error>>{
        let to_comp_file_list = get_file_list(&self.original_path)?;
        let queue = Arc::new(SegQueue::new());
        for i in to_comp_file_list{
            queue.push(i);
        }

        let mut handles = Vec::new();
        let arc_root = Arc::new(self.original_path);
        let arc_dest = Arc::new(self.destination_path);
        for _ in 0..self.thread_count {
            let arc_root = Arc::clone(&arc_root);
            let arc_dest = Arc::clone(&arc_dest);
            let arc_queue = Arc::clone(&queue);
            let arc_cal_func = Arc::clone(&self.calculate_quality_and_size);
            let handle = thread::spawn(move || {
                process(arc_queue, &arc_root, &arc_dest, *arc_cal_func);
            });
            handles.push(handle);
        }

        for h in handles{
            h.join().unwrap();
        }
        return Ok(());
    }
}

fn process(
    queue: Arc<SegQueue<PathBuf>>, 
    root: &PathBuf,
    dest: &PathBuf,
    cal_func: fn(u32, u32, u64) -> Factor){
    while !queue.is_empty() {
        match queue.pop() {
            None => break,
            Some(file) => {
                let file_name = match file.file_name() {
                    None => "",
                    Some(s) => match s.to_str() {
                        None => "",
                        Some(s) => s,
                    },
                };
                let parent = match file.parent(){
                    Some(p) => match p.strip_prefix(root){
                        Ok(p) => p,
                        Err(_) => {
                            println!("Cannot strip the prefix of file {}", file_name);
                            continue;
                        }
                    },
                    None => {
                        println!("Cannot find the parent directory of file {}", file_name);
                        continue;
                    }
                };
                let new_dest_dir = dest.join(parent);
                if !new_dest_dir.is_dir(){
                    match fs::create_dir_all(&new_dest_dir){
                        Ok(_) => {}
                        Err(_) => {
                            println!("Cannot create the parent directory of file {}", file_name);
                            continue;
                        }
                    };
                }
                let compressor = Compressor::new(&file, new_dest_dir, cal_func);
                match compressor.compress_to_jpg(){
                    Ok(_) => {
                        println!("Compress complete! File: {}", file_name);
                    }
                    Err(e) => {
                        println!("Cannot compress image file {} : {}", file_name, e);
                    }
                };
            }
        }
    }
}

fn process_with_sender(
    queue: Arc<SegQueue<PathBuf>>,
    root: &PathBuf,
    dest: &PathBuf,
    cal_func: fn(u32, u32, u64) -> Factor,
    sender: mpsc::Sender<String>){
    while !queue.is_empty() {
        match queue.pop() {
            None => break,
            Some(file) => {
                let file_name = match file.file_name() {
                    None => "",
                    Some(s) => match s.to_str() {
                        None => "",
                        Some(s) => s,
                    },
                };
                let parent = match file.parent(){
                    Some(p) => match p.strip_prefix(root){
                        Ok(p) => p,
                        Err(_) => {
                            println!("Cannot strip the prefix of file {}", file_name);
                            continue;
                        }
                    },
                    None => {
                        println!("Cannot find the parent directory of file {}", file_name);
                        continue;
                    }
                };
                let new_dest_dir = dest.join(parent);
                if !new_dest_dir.is_dir(){
                    match fs::create_dir_all(&new_dest_dir){
                        Ok(_) => {}
                        Err(_) => {
                            println!("Cannot create the parent directory of file {}", file_name);
                            continue;
                        }
                    };
                }
                let compressor = Compressor::new(&file, new_dest_dir, cal_func);
                match compressor.compress_to_jpg(){
                    Ok(p) => {
                        match sender.send(format!("Compress complete! File: {}", p.file_name().unwrap().to_str().unwrap())){
                            Ok(_) => {},
                            Err(e) => {
                                println!("Message passing error!: {}", e);
                            }
                        };
                    }
                    Err(e) => {
                        match sender.send(e.to_string()) {
                            Ok(_) => {},
                            Err(e) => {
                                println!("Message passing error!: {}", e);
                            }
                        };
                    }
                };
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::fs;
    use super::*;

    fn setup(test_num: i32) -> (i32, PathBuf, PathBuf){
        let test_origin_dir = PathBuf::from(&format!("{}{}","test_origin", test_num));
        if test_origin_dir.is_dir(){
            fs::remove_dir_all(&test_origin_dir).unwrap();
        }
        fs::create_dir(&test_origin_dir.as_path()).unwrap();


        let test_dest_dir = PathBuf::from(&format!("{}{}", "test_dest", test_num));
        if test_dest_dir.is_dir(){
            fs::remove_dir_all(&test_dest_dir).unwrap();
        }
        fs::create_dir(&test_dest_dir.as_path()).unwrap();

        let options = CopyOptions::new();
        fs::copy("original_images", &test_origin_dir, &options).unwrap();

        (test_num, test_origin_dir, test_dest_dir)
    }

    fn cleanup(test_num: i32){
        let test_origin_dir = PathBuf::from(&format!("{}{}","test_origin", test_num));
        if test_origin_dir.is_dir(){
            fs::remove_dir_all(&test_origin_dir).unwrap();
        }
        let test_dest_dir = PathBuf::from(&format!("{}{}", "test_dest", test_num));
        if test_dest_dir.is_dir(){
            fs::remove_dir_all(&test_dest_dir).unwrap();
        }
    }

    #[test]
    fn folder_compress_test() {
        let (_, test_origin_dir, test_dest_dir) = setup(4);
        let mut folder_compressor = FolderCompressor::new();
        folder_compress(test_origin_dir, test_dest_dir, 4).unwrap();
        cleanup(4);
    }
}
