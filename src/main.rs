use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::future::join_all;
use rand::{rngs::OsRng, Rng};
use std::sync::Arc;
use wnfs::{
    common::MemoryBlockStore,
    private::{
        forest::{hamt::HamtForest, traits::PrivateForest},
        PrivateDirectory, PrivateFile, PrivateNode,
    },
};

async fn create_file_in_dir(
    mut forest: HamtForest,
    store: MemoryBlockStore,
    content: Vec<u8>,
) -> Arc<PrivateFile> {
    PrivateFile::with_content_rc(
        &forest.empty_name(),
        Utc::now(),
        content,
        &mut forest,
        &store,
        &mut OsRng,
    )
    .await
    .unwrap()
}

async fn get_file(
    dir: Arc<PrivateDirectory>,
    path: Vec<String>,
    forest: HamtForest,
    store: MemoryBlockStore,
) -> Result<Vec<u8>> {
    let file = match dir.get_node(&path, true, &forest, &store).await? {
        Some(PrivateNode::File(file)) => file,
        _ => return Err(anyhow!("Failed to find file")),
    };

    file.get_content(&forest, &store).await
}

static FILES_COUNT: usize = 20;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let store = MemoryBlockStore::default();
    let rng = &mut OsRng;
    let mut forest = HamtForest::new_rsa_2048(rng);
    let mut root = Arc::new(PrivateDirectory::new(&forest.empty_name(), Utc::now(), rng));

    let mut handles = vec![];

    for i in 0..FILES_COUNT {
        let content = format!("{{ \"index\": {}, \"random\": {} }}", i, rng.gen::<f64>());

        handles.push(tokio::spawn(create_file_in_dir(
            forest.clone(),
            store.clone(),
            content.as_bytes().to_vec(),
        )))
    }

    let results = join_all(handles).await;

    // Add the files to the root directory.
    for (i, file) in results.into_iter().flatten().enumerate() {
        let dest = root
            .open_file_mut(
                &[format!("file_{}.json", i)],
                true,
                Utc::now(),
                &mut forest,
                &store,
                &mut OsRng,
            )
            .await
            .unwrap();
        dest.copy_content_from(&file, Utc::now());

        root.as_node()
            .store(&mut forest, &store, &mut OsRng)
            .await
            .unwrap();
    }

    // List all the files from the directory.
    let files = root.ls(&[], true, &forest, &store).await.unwrap();
    println!(">> ls /");
    for (name, meta) in files {
        println!("{} {:?}", name, meta);
    }

    // Get one out of 3 files concurrently.
    let mut files_fut = vec![];
    for i in 0..FILES_COUNT {
        if i % 3 != 0 {
            continue;
        }

        let name = format!("file_{}.json", i);
        files_fut.push(tokio::spawn(get_file(
            Arc::clone(&root),
            vec![name],
            forest.clone(),
            store.clone(),
        )));
    }

    let contents = join_all(files_fut).await;

    // Dump the content of each file
    for content in contents {
        match content {
            Err(err) => {
                println!("ERROR: join failure: {:?}", err);
            }
            Ok(maybe_file) => match maybe_file {
                Err(err) => println!("ERROR: maybe_file failed: {:?}", err),
                Ok(data) => println!("OK: {}", String::from_utf8_lossy(&data)),
            },
        }
    }
}
