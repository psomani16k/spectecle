use std::path::PathBuf;

use crate::{
    actors::ADDRESSES,
    signals::library_signals::{AddToLibrary, DisplayLibrary, LibraryState, UpdateCache},
    utility::state::STATE,
};
use async_trait::async_trait;
use messages::{
    actor::Actor,
    prelude::{Address, Context, Notifiable},
};
use rinf::{DartSignal, RustSignal};
use tokio::{spawn, task::JoinSet};

pub struct LibraryActor {
    _tasks: JoinSet<()>,
}

impl Actor for LibraryActor {}

impl LibraryActor {
    pub async fn create_and_init(ctx: Context<LibraryActor>) -> Address<Self> {
        let mut self_addr = ctx.address();
        let mut owned_tasks = JoinSet::new();
        owned_tasks.spawn(Self::listen_add_to_library(self_addr.clone()));

        spawn(ctx.run(Self { _tasks: owned_tasks }));

        let has_lib = {
            let state = STATE.get().unwrap().read().await;
            state.has_lib()
        };
        if has_lib {
            self_addr.notify(UpdateCache::Refresh).await.unwrap();
        } else {
            LibraryState::NoLibraryAvailable.send_signal_to_dart();
        }
        return self_addr;
    }

    async fn listen_add_to_library(mut self_addr: Address<Self>) {
        let recv = AddToLibrary::get_dart_signal_receiver();
        while let Some(signal) = recv.recv().await {
            let _ = self_addr.notify(signal.message).await;
        }
    }
}

#[async_trait]
impl Notifiable<AddToLibrary> for LibraryActor {
    async fn notify(&mut self, msg: AddToLibrary, _: &Context<Self>) {
        let lib_path = PathBuf::from(msg.path);
        {
            let mut state = STATE.get().unwrap().write().await;
            // TODO: send this to the UI as a pop-up of sorts
            state.import_lib(lib_path).unwrap();
        }
        ADDRESSES
            .get()
            .unwrap()
            .get_library()
            .notify(UpdateCache::Rebuild)
            .await
            .unwrap();
    }
}

#[async_trait]
impl Notifiable<UpdateCache> for LibraryActor {
    async fn notify(&mut self, msg: UpdateCache, _: &Context<Self>) {
        match msg {
            UpdateCache::Refresh => {
                LibraryState::RefreshingCache.send_signal_to_dart();
                {
                    let mut state = STATE.get().unwrap().write().await;
                    state.refresh_cache(false).unwrap();
                }
            }
            UpdateCache::Rebuild => {
                LibraryState::RebuildingCache.send_signal_to_dart();
                {
                    let mut state = STATE.get().unwrap().write().await;
                    state.refresh_cache(true).unwrap();
                }
            }
        };

        let books;

        {
            books = STATE.get().unwrap().read().await.get_book_data();
        }

        LibraryState::Show(DisplayLibrary { data: books }).send_signal_to_dart();
    }
}
