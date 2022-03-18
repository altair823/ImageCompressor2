use std::path::{Path, PathBuf};
use std::env::consts::OS;
use std::error::Error;
use std::io;
use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use subprocess::Exec;
use crossbeam::{queue, thread};
use crossbeam::queue::ArrayQueue;
use image_compressor::crawler::get_dir_list;

fn get_7z_executable_path() -> Result<PathBuf, Box<dyn Error>>{
    // let current_dir = match std::env::current_exe(){
    //     Ok(p) => p.parent().unwrap().to_path_buf(),
    //     Err(_) => return Err(Box::new(io::Error::new(ErrorKind::NotFound, "Cannot get the current directory!"))),
    // };
    // match OS {
    //     "macos" => Ok(current_dir.join(PathBuf::from("7zz"))),
    //     "windows" => Ok(current_dir.join(PathBuf::from("7z.exe"))),
    //     "linux" => Ok(current_dir.join(PathBuf::from("7zzs"))),
    //     e => {
    //         println!("Doesn't support {} currently!", e);
    //         return Err(Box::new(io::Error::new(ErrorKind::NotFound, "Cannot find the 7z executable!")));
    //     }
    // }
    match OS {
        "macos" => Ok(PathBuf::from("./7zz")),
        "windows" => Ok(PathBuf::from("7z.exe")),
        "linux" => Ok(PathBuf::from("./7zzs")),
        e => {
            println!("Doesn't support {} currently!", e);
            return Err(Box::new(io::Error::new(ErrorKind::NotFound, "Cannot find the 7z executable!")));
        }
    }
}

fn compress_a_dir_to_7z(origin: &Path, dest: &Path, root: &Path) ->Result<PathBuf, Box<dyn Error>>{

    let compressor_path = get_7z_executable_path()?;

    let zip_path = match dest.join(&match origin.strip_prefix(root){
        Ok(p) => p,
        Err(_) => origin,
    }).to_str(){
        Some(s) => format!("{}.7z", s),
        None => return Err(Box::new(io::Error::new(ErrorKind::NotFound, "Cannot get the original directory path!"))),
    };

    if Path::new(zip_path.as_str()).is_dir(){
        return Err(Box::new(io::Error::new(ErrorKind::AlreadyExists, "The 7z file is already existed! Abort archiving.")));
    }

    let exec = Exec::cmd(compressor_path)
        .args(&vec!["a", "-mx=9", "-t7z", zip_path.as_str(), match origin.to_str(){
            None => return Err(Box::new(io::Error::new(ErrorKind::NotFound, "Cannot get the destination directory path!"))),
            Some(s) => s,
        }]);
    exec.join()?;
    return Ok(PathBuf::from(zip_path));
}

fn process(queue: &ArrayQueue<PathBuf>, dest_dir: &PathBuf, root: &PathBuf){
    while !queue.is_empty() {
        let dir = match queue.pop() {
            None => break,
            Some(d) => d,
        };
        match compress_a_dir_to_7z(dir.as_path(), &dest_dir, &root){
            Ok(_) => {}
            Err(e) => println!("Error occurred! : {}", e),
        }
    }
}

fn process_with_sender(queue: &ArrayQueue<PathBuf>,
                       dest_dir: &PathBuf,
                       root: &PathBuf,
                       sender: Sender<String>){
    while !queue.is_empty() {
        let dir = match queue.pop() {
            None => break,
            Some(d) => d,
        };
        match compress_a_dir_to_7z(dir.as_path(), &dest_dir, &root){
            Ok(p) => {
                match sender.send(format!("7z archiving complete: {}", p.to_str().unwrap())){
                    Ok(_) => {},
                    Err(e) => println!("Massege passing error!: {}", e),
                }
            }
            Err(e) => println!("Error occurred! : {}", e),
        };
    }
}

pub fn archive_root_dir(root: &Path, dest: &Path, thread_count: u32) -> Result<(), Box<dyn Error>>{
    let to_7z_file_list = match get_dir_list(root){
        Ok(s) => s,
        Err(e) => {
            println!("Cannot extract the list of directories in {} : {}", root.to_str().unwrap(), e);
            return Err(Box::new(e));
        }
    };

    let queue = queue::ArrayQueue::new(to_7z_file_list.len());
    for dir in to_7z_file_list{
        match queue.push(dir){
            Ok(_) => {}
            Err(e) => println!("Cannot push the directory in the queue. : {}", e.to_str().unwrap()),
        };
    }

    //process(&queue, &dest.to_path_buf(), &root.to_path_buf());
    thread::scope(|s|{
        for _ in 0..thread_count{
            s.spawn(|_| {
                process(&queue, &dest.to_path_buf(), &root.to_path_buf());
            });
        }
    }).unwrap();

    Ok(())
}

pub fn archive_root_dir_with_sender(root: &Path,
                                    dest: &Path,
                                    thread_count: u32,
                                    sender: Sender<String>) -> Result<(), Box<dyn Error>>{
    let to_7z_file_list = match get_dir_list(root){
        Ok(s) => s,
        Err(e) => {
            println!("Cannot extract the list of directories in {} : {}", root.to_str().unwrap(), e);
            return Err(Box::new(e));
        }
    };

    match sender.send(format!("Total archive directory count: {}", to_7z_file_list.len())){
        Ok(_) => {},
        Err(e) => println!("Massege passing error!: {}", e),
    }

    let queue = queue::ArrayQueue::new(to_7z_file_list.len());
    for dir in to_7z_file_list{
        match queue.push(dir){
            Ok(_) => {}
            Err(e) => println!("Cannot push the directory in the queue. : {}", e.to_str().unwrap()),
        };
    }

    //process(&queue, &dest.to_path_buf(), &root.to_path_buf());
    thread::scope(|s|{
        for _ in 0..thread_count{
            let new_sender = sender.clone();
            s.spawn(|_| {
                process_with_sender(&queue, &dest.to_path_buf(), &root.to_path_buf(), new_sender);
            });
        }
    }).unwrap();

    match sender.send(String::from("Archiving Complete!")){
        Ok(_) => {},
        Err(e) => println!("Massege passing error!: {}", e),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use fs_extra::dir;
    use fs_extra::dir::CopyOptions;
    use crate::{compress_a_dir_to_7z, archive_root_dir};

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
    fn compress_folder_to_7z_test() {
        let (_, test_origin, test_dest) = setup(5);
        compress_a_dir_to_7z(&test_origin.join("original_images"), &test_dest, &test_origin).unwrap();
        cleanup(5);
    }

    #[test]
    fn compress_root_dir_to_7z_test(){
        let (_, test_origin, test_dest) = setup(6);
        archive_root_dir(&test_origin, &test_dest, 4).unwrap();
    }
}
