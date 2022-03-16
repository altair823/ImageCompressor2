use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use mozjpeg::{ColorSpace, Compress, ScanMode};
use image::imageops::FilterType;

fn convert_to_jpg<'a, O: AsRef<Path> + ?Sized, D: AsRef<Path> + ?Sized>(origin_file: &'a O, dest_dir: &'a D) -> Result<PathBuf, Box<dyn Error>>{
    let img = image::open(&origin_file)?;
    let stem = origin_file.as_ref().file_stem().unwrap();
    let mut new_path = dest_dir.as_ref()
        .join(stem);
    new_path.set_extension("jpg");
    img.save(&new_path)?;

    Ok(new_path)
}

fn compress(resized_img_data: Vec<u8>, target_width: usize, target_height: usize, quality: f32) -> Result<Vec<u8>, String> {
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

fn resize(path: &Path, resize_ratio: f32) -> Result<(Vec<u8>, usize, usize), String> {
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

/// Compress the given image file and save it to target_dir.
/// If the extension of the given image file is not jpg or jpeg, then convert the image to jpg file.
/// If the module can not open the file, just copy it to target_dir.
/// Compress quality and resize ratio calculate based on file size of the image.
/// For a continuous multithreading process, every single error doesn't occur panic or exception and just print error message with return Ok.
///
/// # Examples
/// ```
/// compress_to_jpg(test_origin_dir.join("file4.jpg"), test_dest_dir).unwrap();
///
/// assert!(test_dest_path.is_file());
/// println!("Original file size: {}, Compressed file size: {}",
///     &test_origin_path.metadata().unwrap().len(), test_dest_path.metadata().unwrap().len());
/// ```
pub fn compress_to_jpg<O: AsRef<Path>, D: AsRef<Path>>(origin_file_path: O, target_dir: D) -> Result<(), Box<dyn Error>> {
    let origin_file_path = origin_file_path.as_ref();
    let target_dir = target_dir.as_ref();

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
    let parent = match origin_file_path.parent(){
        None => Path::new(""),
        Some(e) => e,
    };
    if file_extension.ne("jpg") && file_extension.ne("jpeg") {
        match convert_to_jpg(origin_file_path, parent){
            Ok(_) => {},
            Err(e) => {
                println!("Cannot convert file {} to jpg. Just copy it. : {}", file_name, e);
                fs::copy(origin_file_path,
                         target_dir.join(&file_name))?;
                return Ok(());
            },
        };
    }


    let (quality, size_ratio)= match match origin_file_path.metadata(){
        Ok(o) => o.len(),
        Err(e) => {
            println!("Cannot compute the file size of file {}. Default size ratio and quality selected: {}", file_name, e);
            0
        }
    } {
        x if x > 5000000 => (60., 0.5),
        x if x > 1000000 => (65., 0.6),
        x if x > 500000 => (70., 0.6),
        x if x > 300000 => (75., 0.7),
        x if x > 100000 => (80., 0.7),
        _ => (85., 0.8),
    };

    let (resized_img_data, target_width, target_height) = match resize(origin_file_path, size_ratio) {
        Ok(v) => v,
        Err(e) => {
            println!("Cannot resize the image {} : {}", file_name, e);
            return Ok(())
        },
    };
    let compressed_img_data = match compress(resized_img_data, target_width, target_height, quality) {
        Ok(v) => v,
        Err(e) => {
            println!("Cannot compress the image {} : {}", file_name, e);
            return Ok(())
        },
    };

    let mut target_file_name = PathBuf::from(file_stem);
    target_file_name.set_extension("jpg");
    let target_file = target_dir.join(target_file_name);
    let mut file = BufWriter::new(match File::create(target_file) {
        Ok(o) => o,
        Err(e) => {
            println!("Cannot create a buffer of the image file {} : {}", file_name, e);
            return Ok(())
        },
    });
    match file.write_all(&compressed_img_data){
        Ok(o) => o,
        Err(e) => {
            println!("Cannot save the image file {} : {}", file_name, e);
            return Ok(())
        },
    };

    Ok(())
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

        compress_to_jpg(test_origin_dir.join("file4.jpg"), test_dest_dir).unwrap();

        assert!(test_dest_path.is_file());
        println!("Original file size: {}, Compressed file size: {}",
                &test_origin_path.metadata().unwrap().len(), test_dest_path.metadata().unwrap().len());
        cleanup(2);
    }

    #[test]
    fn compress_to_jpg_copy_test(){
        let (_, test_origin_dir, test_dest_dir) = setup(3);
        fs::copy("original_images/file7.txt", test_origin_dir.join("file7.txt")).unwrap();

        compress_to_jpg(test_origin_dir.join("file7.txt"), &test_dest_dir).unwrap();
        assert!(test_dest_dir.join("file7.txt").is_file());
        cleanup(3);
    }

}