use std::sync::OnceLock;

use messages::prelude::{Address, Context};

use crate::actors::library::LibraryActor;

pub mod library;

pub static ADDRESSES: OnceLock<ActorAddresses> = OnceLock::new();

#[derive(Debug)]
pub struct ActorAddresses {
    lib_actor: Address<LibraryActor>,
}

impl ActorAddresses {
    pub fn get_library(&self) -> Address<LibraryActor> {
        return self.lib_actor.clone();
    }
}

pub async fn create_actors() -> anyhow::Result<()> {
    let library_ctx: Context<LibraryActor> = Context::new();
    let library_addr = LibraryActor::create_and_init(library_ctx).await;
    ADDRESSES
        .set(ActorAddresses {
            lib_actor: library_addr,
        })
        .expect("Failed to initialize actors.");
    return anyhow::Ok(());
}
