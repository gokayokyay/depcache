use anyhow::Result;
use futures::StreamExt;
use indicatif::ProgressBar;
use s3::Bucket;
use tokio::{fs::{File, remove_file}, io::AsyncWriteExt};
use tokio_tar::Archive;

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
    let (head_object_result, _) = bucket.head_object(path.clone()).await?;
    let size = head_object_result.content_length;

    let mut response_data_stream = bucket.get_object_stream(path.clone()).await?;

    println!("Downloading dependency archive");
    let bar = ProgressBar::new(size.unwrap().try_into().unwrap());
    let file_name = path.split("/").last().unwrap();
    let mut async_output_file = tokio::fs::File::create(file_name)
        .await
        .expect("Unable to create file");

    while let Some(chunk) = response_data_stream.bytes().next().await {
        let c_size = chunk.len();
        bar.inc(c_size.try_into().unwrap());
        async_output_file.write_all(&chunk).await?;
    }
    let file = File::open(file_name.clone()).await?;
    let mut archive = Archive::new(file);
    archive.unpack(".").await.unwrap();

    rm_cache_archive(file_name.to_string()).await;
    Ok(())
}

async fn rm_cache_archive(file_path: String) {
    remove_file(file_path).await.unwrap();
}