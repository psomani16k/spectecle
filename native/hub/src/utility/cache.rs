use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    hash::{DefaultHasher, Hash, Hasher},
    io::Read,
    path::PathBuf,
    time::UNIX_EPOCH,
};

use anyhow::Ok;
use rinf::debug_print;
use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheItem {
    key: String,
    relative_path: String,
    last_modified: u128,
    title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheData {
    items: HashMap<String, CacheItem>,
}

#[derive(Debug)]
pub struct Cache {
    data: CacheData,
    cache_dir: PathBuf,
}

impl Cache {
    pub fn open(open_lib: PathBuf) -> anyhow::Result<Self> {
        let cache_dir_path = open_lib.join(".spectecle/cache");
        let cache_file_path = cache_dir_path.join("cache.json");
        if cache_file_path.exists() {
            let mut cache_file = File::open(cache_file_path)?;
            let mut content = String::new();
            cache_file.read_to_string(&mut content)?;
            let data: CacheData = serde_json::from_str(&content)?;
            return Ok(Self {
                data,
                cache_dir: cache_dir_path,
            });
        }
        std::fs::create_dir_all(&cache_dir_path)?;
        let cache = Self {
            data: CacheData {
                items: HashMap::new(),
            },
            cache_dir: cache_dir_path,
        };
        let cache_json_string = serde_json::to_string_pretty(&cache.data)?;
        std::fs::write(cache_file_path, cache_json_string)?;
        return Ok(cache);
    }

    pub fn refresh(&mut self, open_lib: PathBuf) -> anyhow::Result<()> {
        let epup_entries = Self::get_epubs(&open_lib)?;
        let mut keys: HashSet<String> = self.data.items.keys().cloned().collect();
        for entry in epup_entries {
            let file_path = entry.path().to_path_buf();
            let rel_path = file_path.strip_prefix(&open_lib)?.to_path_buf();
            let hash = Self::hash_relative_path(&rel_path);
            if !keys.contains(&hash) {
                debug_print!("adding to cache: {:?}", rel_path);
                if let anyhow::Result::Ok(cache_item) = self.cache_file(file_path, rel_path) {
                    self.data.items.insert(cache_item.key.clone(), cache_item);
                }
                continue;
            }
            let last_mod_cache = self.data.items.get(&hash).unwrap().last_modified;
            let last_mod_file = Self::last_modified(&file_path)?;
            if last_mod_cache != last_mod_file {
                debug_print!("updating in cache: {:?}", rel_path);
                self.delete_cover_cache(&hash)?;
                if let anyhow::Result::Ok(cache_item) = self.cache_file(file_path, rel_path) {
                    self.data.items.insert(cache_item.key.clone(), cache_item);
                }
            }
            keys.remove(&hash);
        }

        for key in keys {
            debug_print!(
                "deleting from cache: {:?}",
                self.data.items.get(&key).unwrap().relative_path
            );
            self.delete_cover_cache(&key)?;
            self.data.items.remove(&key);
        }

        self.write_cache_file()?;

        return Ok(());
    }

    pub fn rebuild(&mut self, open_lib: PathBuf) -> anyhow::Result<()> {
        self.clean_cache()?;
        let epup_entries = Self::get_epubs(&open_lib)?;
        for entry in epup_entries {
            let file_path = entry.path().to_path_buf();
            let rel_path = file_path.strip_prefix(&open_lib)?.to_path_buf();
            if let anyhow::Result::Ok(cache_item) = self.cache_file(file_path, rel_path) {
                println!("{}", cache_item.relative_path);
                self.data.items.insert(cache_item.key.clone(), cache_item);
            }
        }
        self.write_cache_file()?;
        return Ok(());
    }

    fn delete_cover_cache(&self, rel_path_hash: &str) -> anyhow::Result<()> {
        let cover_file = self.cache_dir.join(format!("cover/{}", rel_path_hash));
        if cover_file.exists() {
            fs::remove_file(cover_file)?;
        }
        Ok(())
    }

    fn write_cache_file(&self) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(&self.data)?;
        let cache = self.cache_dir.join("cache.json");
        fs::write(cache, content)?;
        Ok(())
    }

    fn clean_cache(&mut self) -> anyhow::Result<()> {
        let cache_file = self.cache_dir.join("cache.json");
        let covers = self.cache_dir.join("covers");
        if cache_file.exists() {
            fs::remove_file(cache_file)?;
        }
        if covers.exists() {
            fs::remove_dir_all(covers)?;
        }
        self.data.items.clear();
        return Ok(());
    }

    fn hash_relative_path(rel_path: &PathBuf) -> String {
        let mut hasher = DefaultHasher::new();
        rel_path.hash(&mut hasher);
        let hash = format!("{:x}", hasher.finish());
        return hash;
    }

    fn last_modified(file_path: &PathBuf) -> anyhow::Result<u128> {
        let md = fs::metadata(&file_path)?;
        let last_modified = md.modified()?;
        let duration = last_modified.duration_since(UNIX_EPOCH)?;
        let last_modified: u128 = duration.as_millis();
        return Ok(last_modified);
    }

    fn cache_file(&self, file_path: PathBuf, rel_path: PathBuf) -> anyhow::Result<CacheItem> {
        let hash = Self::hash_relative_path(&rel_path);
        let last_modified = Self::last_modified(&file_path)?;
        let mut book = epub::doc::EpubDoc::new(&file_path)?;
        if let Some((cover_data, _)) = book.get_cover() {
            let cover_file = self.cache_dir.join(format!("covers/{}", &hash));
            if let Some(cover_dir) = cover_file.parent() {
                if !cover_dir.exists() {
                    fs::create_dir_all(cover_dir)?;
                }
            }
            fs::write(cover_file, cover_data)?;
        }
        let title = if let Some(title) = book.get_title() {
            title
        } else {
            rel_path.file_name().unwrap().to_string_lossy().into_owned()
        };
        return Ok(CacheItem {
            key: hash,
            relative_path: rel_path.to_string_lossy().into_owned(),
            last_modified: last_modified,
            title,
        });
    }

    fn get_epubs(open_lib: &PathBuf) -> anyhow::Result<Vec<DirEntry>> {
        let epub_entries: Vec<DirEntry> = WalkDir::new(open_lib)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("epub"))
            .collect();

        Ok(epub_entries)
    }
}
