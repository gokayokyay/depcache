use std::{process::Command};

use anyhow::{anyhow, Result};
use tokio::fs::metadata;

use crate::{compression::compress_dir};

#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub cargo_file: String,
    pub lock_file: String,
}

pub fn get_crate_info() -> Result<CrateInfo> {
    let cargo_file = match std::fs::read_to_string("Cargo.toml") {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Couldn't read Cargo.toml. Please make sure it has relevant permissions and you're in the same directory");
            eprintln!("{e}");
            return Err(anyhow!(e));
        }
    };
    let crate_toml = toml::from_str::<toml::Value>(&cargo_file).expect("Malformed Cargo.toml file");
    let package_name = &crate_toml["package"]["name"];
    let lock_file = match std::fs::read_to_string("Cargo.lock") {
        Ok(contents) => contents,
        Err(e) => {
            eprintln!("Couldn't read Cargo.lock. Please make sure it has relevant permissions and you're in the same directory and you've built/run the package at least one time.");
            eprintln!("{e}");
            return Err(anyhow!(e));
        }
    };

    return Ok(CrateInfo {
        name: package_name.to_string().replace("\"", ""),
        cargo_file,
        lock_file,
    });
}

pub fn get_crate_hash() -> Result<String> {
    let crate_info = get_crate_info()?;
    let rust_version = get_rust_version();
    let hash = md5::compute(format!(
        "{}||{}||{}",
        rust_version, crate_info.cargo_file, crate_info.lock_file
    ));
    return Ok(format!("{:x}", hash));
}

pub async fn tar_release(target: String, archive_name: String) -> Result<String> {
    match metadata(format!("./target/{target}")).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Failed to read {target} directory.");
            eprintln!("Probably your package doesn't have a release dir. Please run a release command first.");
            eprintln!("{e}");
            return Err(anyhow!("Release dir doesn't exist."));
        }
    };
    let mut target_dirs = vec![];
    target_dirs.push(format!("./target/{target}"));

    for target_dir in target_dirs.clone() {
        swap_crate_dep_files(target_dir.clone()).await?;
    }

    if target.contains("/") {
        let mut splitted: Vec<_> = target.split("/").collect();
        let profile = splitted.pop().unwrap().to_string();
        target_dirs.push(format!("./target/{profile}"));
    }

    compress_dir(target_dirs.clone(), archive_name.to_string()).await;
    for target_dir in target_dirs {
        restore_crate_dep_files(target_dir).await?;
    }
    tokio::fs::remove_dir("./tmpdeps").await?;

    Ok(archive_name.to_string())
}

// Using https://github.com/SergioBenitez/version_check/blob/master/src/lib.rs
pub fn get_rust_version() -> String {
    fn get_version_and_date() -> Option<(Option<String>, Option<String>)> {
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
        Command::new(rustc)
            .arg("--verbose")
            .arg("--version")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| version_and_date_from_rustc_verbose_version(&s))
    }
    fn version_and_date_from_rustc_verbose_version(s: &str) -> (Option<String>, Option<String>) {
        let (mut version, mut date) = (None, None);
        for line in s.lines() {
            let split = |s: &str| s.splitn(2, ":").nth(1).map(|s| s.trim().to_string());
            match line.trim().split(" ").nth(0) {
                Some("rustc") => {
                    let (v, d) = version_and_date_from_rustc_version(line);
                    version = version.or(v);
                    date = date.or(d);
                }
                Some("release:") => version = split(line),
                Some("commit-date:") if line.ends_with("unknown") => date = None,
                Some("commit-date:") => date = split(line),
                _ => continue,
            }
        }

        (version, date)
    }
    fn version_and_date_from_rustc_version(s: &str) -> (Option<String>, Option<String>) {
        let last_line = s.lines().last().unwrap_or(s);
        let mut components = last_line.trim().split(" ");
        let version = components.nth(1);
        let date = components.filter(|c| c.ends_with(')')).next().map(|s| {
            s.trim_end()
                .trim_end_matches(")")
                .trim_start()
                .trim_start_matches('(')
        });
        (version.map(|s| s.to_string()), date.map(|s| s.to_string()))
    }
    let version = get_version_and_date().unwrap();
    return version.0.unwrap();
}

pub async fn swap_crate_dep_files(target_dir: String) -> Result<()> {
    let crate_info: CrateInfo = get_crate_info()?;
    tokio::fs::create_dir_all("./tmpdeps").await?;
    let mut deps_dir = tokio::fs::read_dir(format!("{target_dir}/deps")).await?;
    while let Ok(Some(entry)) = deps_dir.next_entry().await {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(&crate_info.name)
        {
            tokio::fs::rename(
                entry.path(),
                format!("./tmpdeps/{}", entry.file_name().to_string_lossy()),
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn restore_crate_dep_files(target_dir: String) -> Result<()> {
    let mut deps_dir = tokio::fs::read_dir("./tmpdeps").await?;
    while let Ok(Some(entry)) = deps_dir.next_entry().await {
        tokio::fs::rename(
            entry.path(),
            format!(
                "{target_dir}/deps/{}",
                entry.file_name().to_string_lossy()
            ),
        )
        .await?;
    }
    Ok(())
}

pub async fn check_targets() -> Vec<String> {
    let mut target_dir = match tokio::fs::read_dir("./target").await {
        Ok(k) => k,
        Err(_) => return vec![],
    };
    let known_profiles = vec!["debug", "release"];
    let mut targets = vec![];
    while let Ok(Some(entry)) = target_dir.next_entry().await {
        if entry.file_type().await.unwrap().is_dir() {
            if known_profiles.contains(&entry.file_name().to_str().unwrap()) {
                let filename = entry.file_name();
                targets.push(format!("{}", filename.to_string_lossy().to_string()));
            } else {
                let target_platform_dir_name = entry.file_name().clone();
                let target_platform_dir_name = target_platform_dir_name.to_string_lossy();
                let target_platfor_dir_path = format!("./target/{}", target_platform_dir_name);
                let mut target_platform_dir = tokio::fs::read_dir(target_platfor_dir_path.clone())
                .await
                .unwrap();
                while let Ok(Some(entry)) = target_platform_dir.next_entry().await {
                    if entry.file_type().await.unwrap().is_dir() {
                        if known_profiles.contains(&entry.file_name().to_str().unwrap()) {
                            let filename = entry.file_name();
                            targets.push(format!("{}/{}", target_platform_dir_name.clone(), filename.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
    }

    let mut intercepted_targets = vec![];

    for target in targets {
        if target.contains("/debug") {
            let mut splitted: Vec<_> = target.split("/").collect();
            splitted.pop();
            splitted.push("dev");
            intercepted_targets.push(splitted.join("/"));
        } else if target.eq("debug") {
            intercepted_targets.push("dev".to_string());
        }
        intercepted_targets.push(target);
    }

    return intercepted_targets;
}
