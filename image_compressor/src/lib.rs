use std::error::Error;
use std::fs;
use std::path::PathBuf;
use compressor::compress_to_jpg;
use crawler::get_file_list;
use crossbeam::thread;
use crossbeam::queue::ArrayQueue;

mod crawler;
mod compressor;

pub fn folder_compress(root: &PathBuf, dest: &PathBuf, thread_num: i32) -> Result<(), Box<dyn Error>>{
    let to_comp_file_list = get_file_list(&root)?;
    let queue = ArrayQueue::new(to_comp_file_list.len());
    for i in to_comp_file_list{
        queue.push(i).unwrap();
    }
    thread::scope(|s| {
        for _ in 0..thread_num {
            let _handle = s.spawn(|_| {
                process(&queue, &dest, &root);
            });
         }
    }).unwrap();
    //process(&queue, &dest, &root);
    return Ok(());
}

fn process(queue: &ArrayQueue<PathBuf>, dest_dir: &PathBuf, root: &PathBuf){
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
                let new_dest_dir = dest_dir.join(parent);
                if !new_dest_dir.is_dir(){
                    match fs::create_dir_all(&new_dest_dir){
                        Ok(_) => {}
                        Err(_) => {
                            println!("Cannot create the parent directory of file {}", file_name);
                            continue;
                        }
                    };
                }
                match compress_to_jpg(&file, new_dest_dir){
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

#[cfg(test)]
mod tests {
    use std::fs;
    use fs_extra::dir;
    use fs_extra::dir::CopyOptions;
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
        dir::copy("original_images", &test_origin_dir, &options).unwrap();

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
        folder_compress(&test_origin_dir, &test_dest_dir, 10).unwrap();
        //cleanup(4);
    }
}
