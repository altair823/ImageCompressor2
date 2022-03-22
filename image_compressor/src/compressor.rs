//! Containing functions that compress a image. 
//! 
//! # Compressor
//! 
//! The `compress_to_jpg` function resizes the given image and compresses it by a certain percentage. 
//! # Examples
//! ```
//! use std::path::PathBuf;
//! use image_compressor::compressor::Compressor;
//!
//! let origin_dir = PathBuf::from("origin").join("file1.jpg");
//! let dest_dir = PathBuf::from("dest");
//!
//! let compressor = Compressor::new(origin_dir, dest_dir, |width, height, file_size| {return (75., 0.7)});
//! compressor.compress_to_jpg();
//! ```


use std::error::Error;
use std::ffi::OsStr;
use std::{fs, io};
use std::fs::File;
use std::io::{BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};
use mozjpeg::{ColorSpace, Compress, ScanMode};
use image::imageops::FilterType;
use crate::get_file_list;


fn delete_converted_file<O: AsRef<Path>>(file_path: O) -> Result<O, Box<dyn Error>>
        where std::path::PathBuf: PartialEq<O>{
        let current_dir_file_list = match get_file_list(file_path.as_ref().parent().unwrap()){
            Ok(mut v) => {
                if let Some(index) = v.iter().position(|x| *x == file_path){
                    v.remove(index);
                }
                v
            },
            Err(e) => {
                return Err(Box::new(e));
            }
        };


        let current_dir_file_list = current_dir_file_list.iter().map(|p| p.file_stem().unwrap().to_str().unwrap()).collect::<Vec<_>>();
        let t = file_path.as_ref().file_stem().unwrap().to_str().unwrap();
        if !current_dir_file_list.contains(&t){
            return Err(Box::new(io::Error::new(ErrorKind::NotFound,
                                            format!("Cannot delete! The file {} can be the original file. ",
                                                    file_path.as_ref().file_name().unwrap().to_str().unwrap()))))
        }

        match fs::remove_file(&file_path){
            Ok(_) => (),
            Err(e) => return Err(Box::new(e)),
        }

        Ok(file_path)
    }

/// Compressor struct.
/// 
pub struct Compressor<O: AsRef<Path>, D: AsRef<Path>>{
    calculate_quality_and_size: fn(u32, u32, u64) -> (f32, f32),
    original_dir: O,
    destination_dir: D,
}

impl<O: AsRef<Path>, D: AsRef<Path>> Compressor<O, D> {

    /// Create a new compressor. 
    /// 
    /// The new `Compressor` instance needs a function to calculate quality and scaling factor of the new compressed image.
    /// 
    /// # Examples
    /// ```
    /// use std::path::PathBuf;
    /// use image_compressor::compressor::Compressor;
    /// 
    /// let origin_dir = PathBuf::from("origin").join("file1.jpg");
    /// let dest_dir = PathBuf::from("dest");
    /// 
    /// let compressor = Compressor::new(origin_dir, dest_dir, |_, _, _| {return (75., 0.7)});
    /// ```
    pub fn new(origin_dir: O, dest_dir: D, calculator: fn(u32, u32, u64) -> (f32, f32)) -> Self{
        Compressor { calculate_quality_and_size: calculator, original_dir: origin_dir, destination_dir: dest_dir }
    }

    fn convert_to_jpg(&self) -> Result<PathBuf, Box<dyn Error>>{
        let img = image::open(&self.original_dir)?;
        let stem = self.original_dir.as_ref().file_stem().unwrap();
        let mut new_path = match self.original_dir.as_ref().parent(){
            Some(s) => s,
            None => return Err(Box::new(io::Error::new(io::ErrorKind::BrokenPipe, "Cannot get parent directory!"))),
        }
            .join(stem);
        new_path.set_extension("jpg");
        img.save(&new_path)?;

        Ok(new_path)
    }

    fn compress(&self, resized_img_data: Vec<u8>, target_width: usize, target_height: usize, quality: f32) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut comp = Compress::new(ColorSpace::JCS_RGB);
        comp.set_scan_optimization_mode(ScanMode::Auto);
        comp.set_quality(quality);

        comp.set_size(target_width, target_height);

        comp.set_mem_dest();
        comp.set_optimize_scans(true);
        comp.start_compress();

        let mut line = 0;
        loop {
            if line > target_height - 1 {
                break;
            }
            comp.write_scanlines(&resized_img_data[line * target_width * 3..(line + 1) * target_width * 3]);
            line += 1;
        }
        comp.finish_compress();

        let compressed = comp.data_to_vec()
            .map_err(|_| "data_to_vec failed".to_string())?;
        Ok(compressed)
    }

    fn resize(&self, path: &Path, resize_ratio: f32) -> Result<(Vec<u8>, usize, usize), Box<dyn Error>> {
        let img = image::open(path).map_err(|e| e.to_string())?;
        let width = img.width() as usize;
        let height = img.height() as usize;

        let width = width as f32 * resize_ratio;
        let height = height as f32 * resize_ratio;

        let resized_img = img.resize(
            width as u32,
            height as u32,
            FilterType::Triangle);
        Ok((resized_img.to_rgb8().to_vec(), resized_img.width() as usize, resized_img.height() as usize))
    }

    /// Compress a file.
    /// 
    /// Compress the given image file and save it to target_dir.
    /// If the extension of the given image file is not jpg or jpeg, then convert the image to jpg file.
    /// If the module can not open the file, just copy it to target_dir.
    /// Compress quality and resize ratio calculate based on file size of the image.
    /// For a continuous multithreading process, every single error doesn't occur panic or exception and just print error message with return Ok.
    ///
    /// # Examples
    /// ```
    /// use std::path::PathBuf;
    /// use image_compressor::compressor::Compressor;
    ///
    /// let origin_dir = PathBuf::from("origin").join("file1.jpg");
    /// let dest_dir = PathBuf::from("dest");
    ///
    /// let compressor = Compressor::new(origin_dir, dest_dir, |width, height, file_size| {return (75., 0.7)});
    /// compressor.compress_to_jpg();
    /// ```
    pub fn compress_to_jpg(&self) -> Result<PathBuf, Box<dyn Error>> {
        let origin_file_path = self.original_dir.as_ref();
        let target_dir = self.destination_dir.as_ref();

        let file_name = match origin_file_path.file_name(){
            Some(e) => match e.to_str(){
                Some(s) => s,
                None => "",
            },
            None => "",
        };

        let file_stem = origin_file_path.file_stem().unwrap();
        let file_extension = match origin_file_path.extension(){
            None => OsStr::new(""),
            Some(e) => e,
        };

        let mut target_file_name = PathBuf::from(file_stem);
        target_file_name.set_extension("jpg");
        let target_file = target_dir.join(target_file_name);
        if target_dir.join(file_name).is_file(){
            return Err(Box::new(io::Error::new(ErrorKind::AlreadyExists, format!("The file is already existed! file: {}", file_name))))
        }
        if target_file.is_file(){
            return Err(Box::new(io::Error::new(ErrorKind::AlreadyExists, format!("The compressed file is already existed! file: {}", target_file.file_name().unwrap().to_str().unwrap()))))
        }

        let mut converted_file: Option<PathBuf> = None;

        let current_file;
        if file_extension.ne("jpg") && file_extension.ne("jpeg") {
            match self.convert_to_jpg(){
                Ok(p) => {
                    current_file = (&p).to_path_buf();
                    converted_file = Some(p);
                },
                Err(e) => {
                    let m = format!("Cannot convert file {} to jpg. Just copy it. : {}", file_name, e);
                    fs::copy(origin_file_path,
                            target_dir.join(&file_name))?;
                    return Err(Box::new(io::Error::new(ErrorKind::InvalidData, m)))
                },
            };
        }else {
            current_file = self.original_dir.as_ref().to_path_buf();
        }


        print!("{}", current_file.to_str().unwrap());
        let image_file = image::open(&current_file)?;
        let width = image_file.width();
        let height = image_file.height();
        let file_size = match origin_file_path.metadata(){
            Ok(m) => m.len(),
            Err(_) => 0,
        };

        let (quality, size_ratio) = (self.calculate_quality_and_size)(width, height, file_size);

        let (resized_img_data, target_width, target_height) = self.resize(origin_file_path, size_ratio)?;
        let compressed_img_data = self.compress(resized_img_data, target_width, target_height, quality)?;


        let mut file = BufWriter::new(File::create(&target_file)?);
        file.write_all(&compressed_img_data)?;

        match converted_file {
            Some(p) => {
                delete_converted_file(p)?;
            }
            None => {},
        }


        Ok(target_file)
    }
    
}
#[cfg(test)]
mod tests{
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
    fn convert_to_jpg_test(){
        let (_, test_origin_dir, test_dest_dir) = setup(1);

        fs::copy("original_images/file1.png", test_origin_dir.join("file1.png")).unwrap();
        assert_eq!(convert_to_jpg(&test_origin_dir.join("file1.png"), &test_dest_dir).unwrap(),
                   test_dest_dir.join("file1.jpg"));

        fs::copy("original_images/dir1/file5.webp", test_origin_dir.join("file5.webp")).unwrap();
        assert_eq!(convert_to_jpg(&test_origin_dir.join("file5.webp"), &test_dest_dir).unwrap(),
                    test_dest_dir.join("file5.jpg"));
        cleanup(1);
    }

    #[test]
    fn compress_a_image_test(){
        let (_, test_origin_dir, test_dest_dir) = setup(2);
        let test_origin_path = test_origin_dir.join("file4.jpg");
        let test_dest_path = test_dest_dir.join("file4.jpg");

        fs::copy(Path::new("original_images/file4.jpg"), &test_origin_path).unwrap();

        compress_to_jpg(test_origin_dir.join("file4.jpg"), test_dest_dir, |i| -> (f32, f32) { return (75., 0.7);}).unwrap();

        assert!(test_dest_path.is_file());
        println!("Original file size: {}, Compressed file size: {}",
                &test_origin_path.metadata().unwrap().len(), test_dest_path.metadata().unwrap().len());
        cleanup(2);
    }

    #[test]
    fn compress_to_jpg_copy_test(){
        let (_, test_origin_dir, test_dest_dir) = setup(3);
        fs::copy("original_images/file7.txt", test_origin_dir.join("file7.txt")).unwrap();

        assert!(compress_to_jpg(test_origin_dir.join("file7.txt"), &test_dest_dir, |i| -> (f32, f32) { return (75., 0.7);}).is_err());
        assert!(test_dest_dir.join("file7.txt").is_file());
        cleanup(3);
    }

    #[test]
    fn delete_converted_file_test(){
        let (_, test_origin_dir, _) = setup(7);
        //fs::copy("original_images/file1.png", test_origin_dir.join("file1.png")).unwrap();
        fs::copy("original_images/file2.jpg", test_origin_dir.join("file2.jpg")).unwrap();

        match delete_converted_file(test_origin_dir.join("file2.jpg")){
            Ok(o) => println!("{}", o.to_str().unwrap()),
            Err(e) => println!("{}", e),
        }
        cleanup(7);
    }
}