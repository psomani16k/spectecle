use rinf::DartSignal;
use serde::Deserialize;

#[derive(Deserialize, DartSignal)]
pub struct AppSupportDirectory {
    pub path: String,
}
