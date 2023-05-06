use std::env::{self};

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct S3Config {
    pub bucket_name: String,
    pub region: String,
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
}

impl S3Config {
    pub fn init_from_env() -> Result<Self> {
        let bucket_name = env::var("BUCKET_NAME")?;
        let region = env::var("REGION")?;
        let endpoint = env::var("ENDPOINT")?;
        let access_key = env::var("ACCESS_KEY")?;
        let secret_key = env::var("SECRET_KEY")?;
        Ok(Self {
            bucket_name,
            region,
            endpoint,
            access_key,
            secret_key,
        })
    }
}
