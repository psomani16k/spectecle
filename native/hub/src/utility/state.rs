use anyhow::Ok;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::OnceLock;
use tokio::sync::RwLock;

use crate::signals::library_signals::BookData;
use crate::utility::cache::Cache;
use crate::utility::library::Library;

pub static STATE: OnceLock<RwLock<State>> = OnceLock::new();

#[derive(Debug)]
pub struct State {
    support_dir: PathBuf,
    library: Library,
    cache: Option<Cache>,
}

impl State {
    /// Initialize the `LIBRARY` static variable with the library data.
    /// Initializes the data with empty content if needed.
    pub fn initialize(support_dir: String) -> anyhow::Result<()> {
        let support_dir = PathBuf::from_str(&support_dir)?;
        let library = Library::open(&support_dir)?;
        let cache = match library.get_open_lib() {
            Some(open) => Some(Cache::open(open)?),
            None => None,
        };
        let state = RwLock::new(Self {
            support_dir,
            library,
            cache,
        });
        STATE
            .set(state)
            .expect("Failed to set LIBRARY during initialization.");
        Ok(())
    }

    pub fn has_lib(&self) -> bool {
        return self.library.has_lib();
    }

    /// Adds the provided library path as one one of the library options
    /// Does NOT alter the cache.
    pub fn import_lib(&mut self, lib_path: PathBuf) -> anyhow::Result<()> {
        self.library.add_lib_and_switch(lib_path);
        self.library.write(&self.support_dir)?;
        self.cache = Some(Cache::open(self.library.get_open_lib().unwrap())?);
        return anyhow::Ok(());
    }

    pub fn refresh_cache(&mut self, rebuild: bool) -> anyhow::Result<()> {
        let lib = self.library.get_open_lib().unwrap();
        match &mut self.cache {
            Some(cache) => {
                if rebuild {
                    cache.rebuild(lib)
                } else {
                    cache.refresh(lib)
                }
            }
            None => todo!(),
        }
    }

    pub fn get_book_data(&self) -> Vec<BookData> {
        if !self.has_lib() || self.cache.is_none() {
            return vec![];
        }
        return self
            .cache
            .as_ref()
            .unwrap()
            .get_book_data(self.library.get_open_lib().unwrap());
    }
}
