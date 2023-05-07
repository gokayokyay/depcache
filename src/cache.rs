use anyhow::Result;
use futures::StreamExt;
use s3::Bucket;
use tokio::{fs::remove_file, io::AsyncWriteExt};

use crate::compression::decompress_archive;

pub async fn check_cache(bucket: Bucket, path: String) -> bool {
    match bucket.head_object(path).await {
        Ok(_g) => {
            return true;
        }
        Err(e) => {
            eprintln!("{e}");
            return false;
        }
    };
}

pub async fn get_cache(bucket: Bucket, path: String) -> Result<()> {
    let mut response_data_stream = bucket.get_object_stream(path.clone()).await?;

    println!("Downloading dependency archive");
    let file_name = path.split("/").last().unwrap();
    let mut async_output_file = tokio::fs::File::create(file_name)
        .await
        .expect("Unable to create file");

    while let Some(chunk) = response_data_stream.bytes().next().await {
        async_output_file.write_all(&chunk).await?;
    }
    decompress_archive(file_name.to_string()).await;
    rm_cache_archive(file_name.to_string()).await;
    Ok(())
}

async fn rm_cache_archive(file_path: String) {
    remove_file(file_path).await.unwrap();
}
