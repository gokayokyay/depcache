use futures::{future::BoxFuture, FutureExt};
use s3::{Bucket, serde_types::Part};

fn _upload_to_bucket_retry(
    retry_attempt: u32,
    retry_limit: u32,
    bucket: Bucket,
    chunk: Vec<u8>,
    path: String,
    part_number: u32,
    upload_id: String,
) -> BoxFuture<'static, Part> {
    if retry_attempt == retry_limit {
        eprintln!("Retry limit reached while uploading chunk to bucket. Panicking.");
        panic!();
    }
    async move {
        match bucket.put_multipart_chunk(
            chunk.clone(),
            path.as_str(),
            part_number,
            upload_id.as_str(),
            "application/octet-stream"
        ).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error while uploading chunk to bucket. Retrying.");
                eprintln!("{e}");
                return _upload_to_bucket_retry(retry_attempt + 1, retry_limit, bucket, chunk, path, part_number, upload_id).await;
            },
        }
    }.boxed()
}

pub async fn upload_to_bucket_retry(
    retry_limit: u32,
    bucket: Bucket,
    chunk: Vec<u8>,
    path: String,
    part_number: u32,
    upload_id: String,
) -> Part {
    return _upload_to_bucket_retry(0, retry_limit, bucket, chunk, path, part_number, upload_id).await;
}

fn _complete_multipart_upload(
    retry_attempt: u32,
    retry_limit: u32,
    bucket: Bucket,
    path: String,
    upload_id: String,
    parts: Vec<Part>
) -> BoxFuture<'static, ()> {
    if retry_attempt == retry_limit {
        eprintln!("Retry limit reached while completing chunk upload process. Panicking.");
        panic!();
    }

    async move {
        match bucket.complete_multipart_upload(&path, &upload_id, parts.clone()).await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Error while finishing multi chunk upload.");
                eprintln!("{e}");
                eprintln!("Trying again.");
                return _complete_multipart_upload(retry_attempt + 1, retry_limit, bucket, path, upload_id, parts).await;
            },
        };
    }.boxed()
}

pub async fn complete_multipart_upload(
    retry_limit: u32,
    bucket: Bucket,
    path: String,
    upload_id: String,
    parts: Vec<Part>
) {
    _complete_multipart_upload(0, retry_limit, bucket, path, upload_id, parts).await;
}