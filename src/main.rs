use chrono::Utc;
use futures::future::join_all;
use rand::rngs::OsRng;
use wnfs::{
    common::MemoryBlockStore,
    private::{
        forest::{hamt::HamtForest, traits::PrivateForest},
        PrivateFile,
    },
};

async fn create_file_from(mut forest: HamtForest, store: &MemoryBlockStore, content: Vec<u8>) {
    // Create a new file (detached from any directory)
    let _file = PrivateFile::with_content_rc(
        &forest.empty_name(),
        Utc::now(),
        content,
        &mut forest,
        store,
        &mut OsRng,
    )
    .await
    .unwrap();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let store = &MemoryBlockStore::default();
    let rng = &mut OsRng;
    let forest = HamtForest::new_rsa_2048(rng);

    let mut handles = vec![];

    for _i in 0..100 {
        let content = Vec::with_capacity(42 * 1024);
        handles.push(tokio::spawn(create_file_from(
            forest.clone(),
            store,
            content,
        )))
    }

    let results = join_all(handles).await;
}
