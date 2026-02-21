use std::{
    fs,
    io::Read,
    path::PathBuf,
};

use anyhow::Ok;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    open_lib: Option<PathBuf>,
    libraries: Vec<PathBuf>,
}

impl Library {
    pub fn open(support_dir: &PathBuf) -> anyhow::Result<Library> {
        let lib_file = support_dir.join("lib.json");
        if lib_file.exists() {
            let mut file = fs::File::open(&lib_file)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            let data: Library = serde_json::from_str(&contents)?;
            return Ok(data);
        }
        let data = Library {
            open_lib: None,
            libraries: Vec::new(),
        };
        let lib_json_string = serde_json::to_string_pretty(&data)?;
        std::fs::write(lib_file, lib_json_string)?;
        return Ok(data);
    }

    pub fn write(&self, support_dir: &PathBuf) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        let lib = support_dir.join("lib.json");
        std::fs::write(lib, content)?;
        return Ok(());
    }

    pub fn get_open_lib(&self) -> Option<PathBuf> {
        return self.open_lib.clone();
    }

    pub fn has_lib(&self) -> bool {
        return self.open_lib.is_some();
    }

    fn add_lib(&mut self, lib_path: PathBuf) {
        self.libraries.push(lib_path);
    }

    pub fn add_lib_and_switch(&mut self, lib_path: PathBuf) {
        self.add_lib(lib_path.clone());
        self.open_lib = Some(lib_path);
    }
}
