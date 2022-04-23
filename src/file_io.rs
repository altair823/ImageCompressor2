use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use serde_json::{from_reader, to_writer_pretty};
use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize)]
pub enum DataType{
    Directory(Option<PathBuf>),
    Number(Option<i32>),
    Boolean(Option<bool>),
    String(Option<String>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProgramData {
    data: HashMap<String, DataType>,
}

impl ProgramData {
    pub fn new() -> Self{
        ProgramData {
            data: Default::default(),
        }
    }

    pub fn set_data(&mut self, key: &str, value: DataType){
        self.data.insert(key.to_string(), value);
    }

    pub fn get_data(&self, key: &str) -> Option<&DataType>{
        self.data.get(key)
    }

    pub fn save<O: AsRef<Path>>(&self, file_path: O) -> Result<O, Box<dyn Error>>{
        //let file_path = Path::new(&file_path);
        match file_path.as_ref().parent() {
            Some(p) => fs::create_dir_all(p)?,
            None => {},
        }

        let save_file = File::create(&file_path)?;
        if let Err(e) =  to_writer_pretty(&save_file, &self){
            return Err(Box::new(e))
        }

        Ok(file_path)
    }

    pub fn load<O: AsRef<Path>>(file_path: O) -> Result<ProgramData, Box<dyn Error>>{
        let save_file= File::open(file_path)?;
        let json_value = from_reader(BufReader::new(save_file))?;

        return Ok(json_value);
    }
}

impl Default for ProgramData {
    fn default() -> Self {
        ProgramData {
            data: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::DEFAULT_SAVE_FILE_PATH;
    use super::*;

    fn make_dir_set() -> ProgramData {
        let mut dir_set = ProgramData::new();
        dir_set.set_data("origin", DataType::Directory(Some(PathBuf::from("test_origin"))));
        dir_set.set_data("destination", DataType::Directory(Some(PathBuf::from("test_dest"))));
        dir_set.set_data("archive", DataType::Directory(Some(PathBuf::from("test_archive"))));
        dir_set
    }

    #[test]
    fn save_test(){
        let dir_set = make_dir_set();
        dir_set.save( DEFAULT_SAVE_FILE_PATH).unwrap();
        assert!(Path::new(DEFAULT_SAVE_FILE_PATH).is_file())
    }

    #[test]
    fn load_test(){
        let dir_set = make_dir_set();
        dir_set.save(Path::new(DEFAULT_SAVE_FILE_PATH)).unwrap();
        let json_value = ProgramData::load(DEFAULT_SAVE_FILE_PATH).unwrap();
        println!("{:?}", json_value);
    }
}