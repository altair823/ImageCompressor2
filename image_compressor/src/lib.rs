use std::error::Error;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use compressor::compress_to_jpg;
use crawler::get_file_list;
use std::thread;
use crossbeam_queue::{ArrayQueue, SegQueue};

pub mod crawler;
pub mod compressor;

pub fn folder_compress_with_channel(root: PathBuf,
                                    dest: PathBuf,
                                    thread_num: u32,
                                    sender: mpsc::Sender<String>) -> Result<(), Box<dyn Error>> {
    let to_comp_file_list = get_file_list(&root)?;
    match sender.send(format!("Total file count: {}", to_comp_file_list.len())) {
        Ok(_) => {},
        Err(e) => {
            println!("Massege passing error!: {}", e);
        }
    }

    let queue = Arc::new(SegQueue::new());
    for i in to_comp_file_list{
        queue.push(i);
    }
    let mut handles = Vec::new();
    let arc_root = Arc::new(root);
    let arc_dest = Arc::new(dest);
    for _ in 0..thread_num {
        let new_sender = sender.clone();
        let arc_root = Arc::clone(&arc_root);
        let arc_dest = Arc::clone(&arc_dest);
        let arc_queue = Arc::clone(&queue);
        let handle = thread::spawn(move || {
            process_with_sender(arc_queue, &arc_dest, &arc_root, new_sender);
        });
        handles.push(handle);
    }

    for h in handles{
        h.join().unwrap();
    }
    match sender.send(String::from("Compress complete!")){
        Ok(_) => {},
        Err(e) => {
            println!("Massege passing error!: {}", e);
        }
    };
    // let new_sender = mpsc::Sender::clone(&sender);
    // process_with_sender(&queue, &dest, &root, new_sender);
    return Ok(());
}

/// duplicated.
pub fn folder_compress(root: &PathBuf, dest: &PathBuf, thread_num: u32) -> Result<(), Box<dyn Error>>{
    let to_comp_file_list = get_file_list(&root)?;
    let queue = ArrayQueue::new(to_comp_file_list.len());
    for i in to_comp_file_list{
        queue.push(i).unwrap();
    }
    // thread::scope(|s| {
    //     for _ in 0..thread_num {
    //         let _handle = s.spawn(|_| {
    //             process(&queue, &dest, &root);
    //         });
    //      }
    // }).unwrap();
    //process(&queue, &dest, &root);
    return Ok(());
}

fn process_with_sender(queue: Arc<SegQueue<PathBuf>>,
                       dest_dir: &PathBuf,
                       root: &PathBuf,
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
                    Ok(p) => {
                        match sender.send(format!("Compress complete! File: {}", p.file_name().unwrap().to_str().unwrap())){
                            Ok(_) => {},
                            Err(e) => {
                                println!("Massege passing error!: {}", e);
                            }
                        };
                    }
                    Err(e) => {
                        match sender.send(e.deref().to_string()) {
                            Ok(_) => {},
                            Err(e) => {
                                println!("Massege passing error!: {}", e);
                            }
                        };
                    }
                };
            }
        }
    }
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
        cleanup(4);
    }
}
