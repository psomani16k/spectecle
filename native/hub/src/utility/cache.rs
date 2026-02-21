use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    hash::{DefaultHasher, Hash, Hasher},
    io::{Cursor, Read},
    path::{Component, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::Ok;
use fast_image_resize::{IntoImageView, Resizer};
use image::{DynamicImage, ImageFormat, ImageReader, RgbaImage};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use rinf::debug_print;
use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use crate::signals::library_signals::BookData;

const BATCH_SIZE: usize = 48;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheItem {
    key: String,
    relative_path: String,
    last_modified: u128,
    title: String,
    has_cover: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheData {
    items: HashMap<String, CacheItem>,
}

#[derive(Debug)]
pub struct Cache {
    data: CacheData,
    cache_dir: PathBuf,
    covers: HashMap<String, Vec<u8>>,
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
                covers: HashMap::new(),
            });
        }
        std::fs::create_dir_all(&cache_dir_path)?;
        let cache = Self {
            data: CacheData {
                items: HashMap::new(),
            },
            cache_dir: cache_dir_path,
            covers: HashMap::new(),
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
                if let anyhow::Result::Ok(cache_item) = self.cache_file(file_path, rel_path) {
                    self.data.items.insert(cache_item.key.clone(), cache_item);
                }
                continue;
            }
            let last_mod_cache = self.data.items.get(&hash).unwrap().last_modified;
            let last_mod_file = Self::last_modified(&file_path)?;
            if last_mod_cache != last_mod_file {
                self.delete_cover_cache(&hash)?;
                if let anyhow::Result::Ok(cache_item) = self.cache_file(file_path, rel_path) {
                    self.data.items.insert(cache_item.key.clone(), cache_item);
                }
            }
            keys.remove(&hash);
        }

        for key in keys {
            self.delete_cover_cache(&key)?;
            self.data.items.remove(&key);
        }

        self.write_cache_file()?;
        self.write_covers_par(true)?;

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
        self.write_covers_par(true)?;
        return Ok(());
    }

    pub fn get_book_data(&self, open_lib: PathBuf) -> Vec<BookData> {
        let book_data: Vec<BookData> = self
            .data
            .items
            .iter()
            .map(|entry| {
                let entry = entry.1;
                let key = entry.key.clone();
                let book_path = open_lib
                    .join(&entry.relative_path)
                    .to_string_lossy()
                    .into_owned();
                let cover_path = match entry.has_cover {
                    true => {
                        let cover_path = self.cache_dir.join(format!("covers/{}", &key));
                        let cover_path = cover_path.to_string_lossy().into_owned();
                        Some(cover_path)
                    }
                    false => None,
                };
                let title = entry.title.clone();
                return BookData {
                    key,
                    book_path,
                    cover_path,
                    title,
                };
            })
            .collect();

        return book_data;
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

    fn cache_file(&mut self, file_path: PathBuf, rel_path: PathBuf) -> anyhow::Result<CacheItem> {
        let hash = Self::hash_relative_path(&rel_path);
        let last_modified = Self::last_modified(&file_path)?;
        let mut has_cover = false;
        let mut book = epub::doc::EpubDoc::new(&file_path)?;
        if let Some((cover_data, _)) = book.get_cover() {
            self.push_cover_for_writing(&hash, cover_data);
            has_cover = true;
        }
        if !has_cover {
            let mut cover_spine = None;
            for spine in &book.spine {
                if spine.idref.contains("cover") && !spine.idref.contains("back") {
                    cover_spine = Some(spine.clone());
                }
            }
            let mut img_path = None;
            match &cover_spine {
                Some(cover_spine) => {
                    if let Some((cover_page_data, _)) = book.get_resource(&cover_spine.idref) {
                        let cover_page = String::from_utf8_lossy(&cover_page_data).to_string();
                        let re = Regex::new(
                            r#"(?i)<(?:img|image)[^>]+(?:src|href)\s*=\s*["']([^"']+)["']"#,
                        )
                        .unwrap();
                        if let Some(img_path_matches) = re.captures(&cover_page) {
                            if let Some(img_path_match) = img_path_matches.get(1) {
                                img_path = Some(img_path_match.as_str().to_string());
                            }
                        }
                    }
                }
                None => {}
            }
            if let Some(mut img_path) = img_path {
                if img_path.starts_with("../") {
                    let cover_page_path = book
                        .resources
                        .get(&cover_spine.unwrap().idref)
                        .unwrap()
                        .path
                        .clone();
                    let abs_path = cover_page_path.parent().unwrap();
                    let img_path =
                        Self::normalise_img_path(abs_path.to_path_buf(), PathBuf::from(img_path));

                    if let Some(img_path) = img_path {
                        if let Some(cover_data) = book.get_resource_by_path(&img_path) {
                            self.push_cover_for_writing(&hash, cover_data);
                            has_cover = true;
                        }
                    }
                } else {
                    img_path = String::from("OEBPS/") + &img_path;
                    if let Some(cover_data) = book.get_resource_by_path(&img_path) {
                        self.push_cover_for_writing(&hash, cover_data);
                        has_cover = true;
                    }
                }
            }
        }
        if !has_cover {
            let mut cover_id = None;
            for (key, item) in &book.resources {
                if key.to_ascii_lowercase().contains("cover")
                    && !key.to_ascii_lowercase().contains("back")
                    && item.mime.starts_with("image/")
                    && !item.mime.contains("html")
                {
                    cover_id = Some(key.clone());
                }
            }
            match cover_id {
                Some(id) => {
                    let cover = book.get_resource(&id);
                    if cover.is_some() {
                        let (cover_data, _) = cover.unwrap();
                        self.push_cover_for_writing(&hash, cover_data);
                        has_cover = true;
                    }
                }
                None => {}
            }
        }
        if !has_cover {
            let mut cover = None;
            for md in &book.metadata {
                if md.property == "cover" || md.property == "cover-image" {
                    cover = Some(md.value.clone());
                }
            }
            if let Some(cover_path_internal) = cover {
                match book.get_resource_by_path(&cover_path_internal) {
                    Some(cover_data) => {
                        self.push_cover_for_writing(&hash, cover_data);
                        has_cover = true;
                    }
                    None => {}
                }
            }
        }
        let title = book
            .get_title()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                rel_path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap()
            });
        self.write_covers_par(false)?;
        return Ok(CacheItem {
            key: hash,
            relative_path: rel_path.to_string_lossy().into_owned(),
            last_modified: last_modified,
            title,
            has_cover,
        });
    }

    fn push_cover_for_writing(&mut self, hash: &String, cover_data: Vec<u8>) {
        self.covers.insert(hash.clone(), cover_data);
    }

    fn write_covers_par(&mut self, force: bool) -> anyhow::Result<()> {
        if force || self.covers.len() >= BATCH_SIZE {
            debug_print!("{}", self.covers.len());
            let errors: Vec<anyhow::Result<()>> = self
                .covers
                .par_iter()
                .map(|(hash, data)| -> anyhow::Result<()> { self.write_cover_file(hash, data) })
                .collect();
            self.covers.clear();
            errors.into_iter().for_each(|e| e.unwrap());
        }
        Ok(())
    }

    fn write_cover_file(&self, hash: &str, data: &Vec<u8>) -> anyhow::Result<()> {
        let cover_file = self.cache_dir.join(format!("covers/{}", &hash));
        if let Some(cover_dir) = cover_file.parent() {
            if !cover_dir.exists() {
                fs::create_dir_all(cover_dir)?;
            }
        }
        let src_img = ImageReader::new(Cursor::new(data)).with_guessed_format()?;
        let img = src_img.decode()?.to_rgba8();
        let width = img.width();
        let height = img.height();
        let target_height = 300;
        if height <= target_height {
            fs::write(cover_file, data)?;
            return Ok(());
        }
        let aspect_ratio = width as f32 / height as f32;
        let target_width = (target_height as f32 * aspect_ratio).round() as u32;
        let mut target_img = fast_image_resize::images::Image::new(
            target_width,
            target_height,
            img.pixel_type().unwrap(),
        );

        let mut resizer = Resizer::new();
        resizer.resize(&img, &mut target_img, None)?;
        let target_img = target_img.into_vec();
        let rgba_img = RgbaImage::from_raw(target_width, target_height, target_img);
        let mut result_buf = Vec::new();
        match rgba_img {
            Some(img_buff) => {
                let final_img = DynamicImage::ImageRgba8(img_buff);
                final_img.write_to(&mut Cursor::new(&mut result_buf), ImageFormat::Jpeg)?;
                fs::write(cover_file, result_buf)?;
                return Ok(());
            }
            None => {
                fs::write(cover_file, data)?;
                return Ok(());
            }
        };
    }

    fn normalise_img_path(abs_dir_path: PathBuf, rel_path: PathBuf) -> Option<PathBuf> {
        let mut normalised: Vec<Component> = abs_dir_path.components().collect();
        let components = rel_path.components();
        for comp in components {
            if comp.as_os_str() == ".." && normalised.is_empty() {
                return None;
            } else if comp.as_os_str() == ".." {
                normalised.pop();
            } else {
                normalised.push(comp);
            }
        }
        let final_path: PathBuf = normalised.iter().collect();
        return Some(final_path);
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
