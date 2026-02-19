use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, DartSignal)]
pub struct AddToLibrary {
    pub path: String,
}

#[derive(Deserialize, DartSignal)]
pub enum UpdateCache {
    Refresh,
    Rebuild,
}

#[derive(Serialize, RustSignal)]
pub enum LibraryState {
    Show(DisplayLibrary),
    NoLibraryAvailable,
    RefreshingCache,
    RebuildingCache,
}

#[derive(Serialize, SignalPiece)]
pub struct DisplayLibrary {
    pub data: Vec<BookData>,
}

#[derive(Serialize, SignalPiece)]
pub struct BookData {
    pub key: String,
    pub book_path: String,
    pub cover_path: Option<String>,
    pub title: String,
}
