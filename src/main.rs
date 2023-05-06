mod s3config;
mod platform;
mod crate_utils;
mod upload_utils;

use anyhow::Result;
use s3::creds::Credentials;
use s3::Bucket;
use tokio::fs;

use crate::crate_utils::tar_release;
use crate::platform::get_platform_hash;
use crate::upload_utils::{upload_to_bucket_retry, complete_multipart_upload};

static CHUNK_SIZE: usize = 100_000_000;

#[tokio::main]
async fn main() -> Result<()> {
    let platform_hash = get_platform_hash()?;
    let config = match s3config::S3Config::init_from_env() {
        Ok(c) => c,
        Err(_) => {
            eprintln!("An environment value not found. Please make sure that you have BUCKET_NAME, REGION, ENDPOINT, ACCESS_KEY, SECRET_KEY values configured.");
            panic!();
        }
    };
    let pkg_name = crate_utils::get_crate_info()?.name;
    let pkg_hash = crate_utils::get_crate_hash()?;
    
    let release_tar_file_name = tar_release().await.unwrap();
    let s3_path = format!("{pkg_name}/{platform_hash}/{pkg_hash}/{release_tar_file_name}");
    println!("{s3_path}");
    // This requires a running minio server at localhost:9000
    let bucket = Bucket::new(
        &config.bucket_name,
        s3::region::Region::Custom {
            region: config.region,
            endpoint: config.endpoint,
        },
        Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        )
        .unwrap(),
    )?
    .with_path_style();

    let tar_data = fs::read(release_tar_file_name).await.unwrap();
    let multi_init_resp = bucket.initiate_multipart_upload(&s3_path, "application/octet-stream").await?;
    let chunks = tar_data.chunks(CHUNK_SIZE).into_iter();

    println!("Uploading the file");

    let mut parts = vec![];

    for (idx, chunk) in chunks.enumerate() {
        println!("Uploading the chunk no {idx}");
        let part = upload_to_bucket_retry(10, bucket.clone(),  chunk.to_vec(), s3_path.clone(), idx as u32 + 1, multi_init_resp.upload_id.clone()).await;
        parts.push(part);
    }
    complete_multipart_upload(10, bucket.clone(), s3_path.clone(), multi_init_resp.upload_id.clone(), parts).await;

    let (head_object_result, code) = bucket.head_object(s3_path.clone()).await?;
    assert_eq!(code, 200);
    assert_eq!(
        head_object_result.content_type.unwrap_or_default(),
        "application/octet-stream".to_owned()
    );

    println!("{}", head_object_result.content_length.unwrap());

    Ok(())
}
