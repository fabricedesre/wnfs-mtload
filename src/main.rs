use anyhow::Result;
use chrono::Utc;
use futures::future::join_all;
use rand::{rngs::OsRng, Rng};
use std::sync::Arc;
use wnfs::{
    common::MemoryBlockStore,
    private::{
        forest::{hamt::HamtForest, traits::PrivateForest},
        PrivateDirectory, PrivateFile,
    },
};

async fn create_file_in_dir(
    mut dir: Arc<PrivateDirectory>,
    mut forest: HamtForest,
    store: MemoryBlockStore,
    name: String,
    content: Vec<u8>,
) -> Result<()> {
    let now = Utc::now();
    let source = PrivateFile::with_content_rc(
        &forest.empty_name(),
        now,
        content,
        &mut forest,
        &store,
        &mut OsRng,
    )
    .await
    .unwrap();

    let dest = dir
        .open_file_mut(&[name], true, Utc::now(), &mut forest, &store, &mut OsRng)
        .await?;
    dest.copy_content_from(&source, now);

    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let store = MemoryBlockStore::default();
    let rng = &mut OsRng;
    let forest = HamtForest::new_rsa_2048(rng);
    let root = Arc::new(PrivateDirectory::new(&forest.empty_name(), Utc::now(), rng));

    let mut handles = vec![];

    for i in 0..100 {
        let content = format!("{{ \"index\": {}, \"random\": {} }}", i, rng.gen::<f64>());

        handles.push(tokio::spawn(create_file_in_dir(
            Arc::clone(&root),
            forest.clone(),
            store,
            format!("file_{}.json", i),
            content.as_bytes().to_vec(),
        )))
    }

    let results = join_all(handles).await;
}
