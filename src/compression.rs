use tokio::process::Command;

pub fn archive_name(target: String) -> String {
    let name = target.replace("/", "_");
    format!("{name}.tar.gz")
}

pub async fn check_tar() {
    let mut cmd = Command::new("tar");
    cmd.arg("--version");
    match cmd.output().await {
        Ok(_) => println!("tar found"),
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::NotFound) {
                println!("`tar` was not found! Check your PATH!")
            } else {
                println!("Error while issuing tar");
            }
            eprintln!("{e}");
            panic!();
        }
    }
}

pub async fn compress_dir(dir_paths: Vec<String>, output_path: String) {
    let mut cmd = Command::new("tar");
    cmd.arg("-czvf");
    cmd.arg(output_path.as_str());
    for dir_path in dir_paths {
        cmd.arg(dir_path.as_str());
    }

    let child = cmd.spawn().unwrap();
    let res = child.wait_with_output().await.unwrap();
    println!("{:?}", res);
}

pub async fn decompress_archive(archive_path: String) {
    let mut cmd = Command::new("tar");
    cmd.args(["-xvzf", archive_path.as_str()]);
    let child = cmd.spawn().unwrap();
    let res = child.wait_with_output().await.unwrap();
    println!("{:?}", res);
}
