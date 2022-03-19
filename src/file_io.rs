use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use serde_json::{from_reader, json, Value, to_writer_pretty};
use serde::Deserialize;

pub const DIR_HISTORY_FILE_NAME: &str = "dir_history.json";
pub const DEFAULT_SAVE_FILE_PATH: &str = "data";

pub enum DirType{
    Origin,
    Destination,
    Archive,
}

#[derive(Debug, Deserialize)]
pub struct DirSet{
    origin_dir: Option<PathBuf>,
    dest_dir: Option<PathBuf>,
    archive_dir: Option<PathBuf>,
}

impl DirSet {
    pub fn new() -> Self{
        DirSet {
            origin_dir: Some(PathBuf::from("")),
            dest_dir: Some(PathBuf::from("")),
            archive_dir: Some(PathBuf::from("")),
        }
    }

    pub fn set_dir<O: AsRef<Path>>(&mut self, dir_type: DirType, dir: O){
        match dir_type {
            DirType::Origin => self.origin_dir = Some(PathBuf::from(dir.as_ref())),
            DirType::Destination => self.dest_dir = Some(PathBuf::from(dir.as_ref())),
            DirType::Archive => self.archive_dir = Some(PathBuf::from(dir.as_ref())),
        }
    }

    pub fn get_dir(&self, dir_type: DirType) -> Option<PathBuf>{
        match dir_type {
            DirType::Origin => self.origin_dir.clone(),
            DirType::Destination => self.dest_dir.clone(),
            DirType::Archive => self.archive_dir.clone(),
        }
    }

    pub fn save_dir_history(&self) -> Result<PathBuf, Box<dyn Error>>{
        let save_file_path = PathBuf::from(DEFAULT_SAVE_FILE_PATH);
        fs::create_dir_all(&save_file_path)?;

        let new_save_file = save_file_path.join(DIR_HISTORY_FILE_NAME);
        let save_file = File::create(&new_save_file)?;
        let json_data = self.make_json_data();
        if let Err(e) =  to_writer_pretty(save_file, &json_data){
            return Err(Box::new(e))
        }

        Ok(new_save_file.to_path_buf())
    }

    fn make_json_data(&self) -> Value {
        let json_data = json! ({"origin_dir": match &self.origin_dir{
                Some(p) => match p.to_str(){
                    Some(s) => s,
                    None => "",
                },
                None => "",
            },
            "dest_dir": match &self.dest_dir {
                Some(p) => match p.to_str() {
                    Some(s) => s,
                    None => "",
                },
                None => "",
            },
            "archive_dir": match &self.archive_dir {
                Some(p) => match p.to_str() {
                    Some(s) => s,
                    None => "",
                },
                None => "",
            }
        });
        json_data
    }

    pub fn load_dir_history() -> Result<DirSet, Box<dyn Error>>{
        let save_file= File::open(Path::new(DEFAULT_SAVE_FILE_PATH).join(DIR_HISTORY_FILE_NAME))?;

        let json_value = from_reader(BufReader::new(save_file))?;

        return Ok(json_value);
    }
}

impl Default for DirSet {
    fn default() -> Self {
        DirSet {
            origin_dir: None,
            dest_dir: None,
            archive_dir: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dir_set() -> DirSet{
        let mut dir_set = DirSet::new();
        dir_set.set_dir(DirType::Origin, "test_origin");
        dir_set.set_dir(DirType::Destination, "test_dest");
        dir_set.set_dir(DirType::Archive, "test_archive");
        dir_set
    }

    #[test]
    fn json_value_test(){
        let dir_set = make_dir_set();
        println!("{}", dir_set.make_json_data().to_string());
    }

    #[test]
    fn save_test(){
        let dir_set = make_dir_set();
        dir_set.save_dir_history().unwrap();
        //assert!(dir_set.save_file_dir.unwrap().join(DIR_HISTORY_FILE_NAME).is_file())
    }

    #[test]
    fn load_test(){
        let dir_set = make_dir_set();
        dir_set.save_dir_history().unwrap();
        let json_value = DirSet::load_dir_history().unwrap();
        println!("{:?}", json_value);
    }
}