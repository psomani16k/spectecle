//! This `hub` crate is the
//! entry point of the Rust logic.

mod actors;
mod signals;
mod utility;

use rinf::{DartSignal, dart_shutdown, write_interface};

use crate::{
    actors::create_actors,
    signals::{library_signals::UpdateCache, utility_signals::AppSupportDirectory},
    utility::state::State,
};

write_interface!();

// You can go with any async library, not just `tokio`.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let receiver = AppSupportDirectory::get_dart_signal_receiver();
    if let Some(support_dir) = receiver.recv().await {
        State::initialize(support_dir.message.path).unwrap();
    }
    create_actors().await.unwrap();
    
    // Keep the main function running until Dart shutdown.
    dart_shutdown().await;
}
