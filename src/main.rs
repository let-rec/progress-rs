use std::{fs::File, process::Stdio};
use futures_util::StreamExt;
use futures::TryFutureExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{self, Client, Response, RequestBuilder};
use tokio_stream::Stream;
use std::io::{prelude::*, Bytes};
use std::cmp::min;


use std::io::BufReader;
use std::time::Instant;
use std::path::Path;

#[tokio::main]
async fn main() {
    let flake_url = "github:msftyago/nix#nixosConfigurations.yago.config.system.build.toplevel";
    // let flake_url = "nixpkgs#go";
    let client = Client::new();

    let child = tokio::process::Command::new("nix")
        .args(["build", flake_url, "--dry-run"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let out = child.wait_with_output().await.unwrap();

    let (fetched, _built, _) = String::try_from(out.stderr)
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .fold((Vec::new(), Vec::new(), false), parse_dry_lines);
    for fetch in fetched.iter() {
        println!("Iter fetched: {fetch:?}");
        download(&client,fetch, "/home/letrec/workspace/self/progress-rs/tmp").await;
    }
}

fn parse_nar_hash(line: &str) -> Option<&str> {
    line.split("/").nth(3)?.split("-").next()
}

fn parse_dry_lines(
    (mut fetched, mut built, is_fetched): (Vec<String>, Vec<String>, bool),
    line: String,
) -> (Vec<String>, Vec<String>, bool) {
    if line.contains("will be fetched") {
        return (fetched, built, true);
    }

    if line.contains("will be built") {
        return (fetched, built, false);
    }

    if let Some(hash) = parse_nar_hash(&line) {
        if is_fetched {
            fetched.push(hash.to_string());
        } else {
            built.push(hash.to_string());
        }
    }

    (fetched, built, is_fetched)
}

enum DownloadProgress {
    Downloaded,
    Downloading,
    Error
}

/*
async fn download(client: &Client, nar: &str) -> Result<DownloadProgress, reqwest::StatusCode> {
    let narinfo = client.get(format!("https://cache.xinux.uz/{nar}.narinfo"))
        .send()
        .await
        .unwrap();
    let nar = narinfo.text().await.unwrap();
    let narinfo = sui_compat::narinfo::NarInfo::parse(&nar).unwrap();

    // let content = client.get(&format!("https://cache.xinux.uz/{}", narinfo.url))
    //     .send()
    //     .await
    //     .unwrap();

    let dwn = download_file(&client, &format!("https://cache.xinux.uz/{}", narinfo.url), "/home/letrec/workspace/self/progress-rs/tmp").await;
    Ok(DownloadProgress::Downloaded)

    // if content.status().is_success() {
    //     let dwn = download_file(&client, &format!("https://cache.xinux.uz/{}", narinfo.url), "/home/letrec/workspace/self/progress-rs/tmp").await;
    //     Ok(DownloadProgress::Downloaded)
    // } else {
    //     match content.error_for_status() {
    //         Ok(res) => {
    //             println!("Status res: {:?}", res);
    //             Ok(DownloadProgress::Error)
    //         },
    //         Err(err) => {
    //             println!("Error res: {:?}", err.status());
    //             Err(reqwest::StatusCode::BAD_REQUEST)
    //         }
    //     }
    // }
}
*/


pub async fn download(client: &Client, nar:&str, path: &str) -> Result<(), String> {
    let narinfo = client.get(format!("https://cache.xinux.uz/{nar}.narinfo"))
        .send()
        .await
        .unwrap();
    let nar = narinfo.text().await.unwrap();
    let narinfo = sui_compat::narinfo::NarInfo::parse(&nar).unwrap();
    let url = &format!("https://cache.xinux.uz/{}", narinfo.url);

    // Reqwest setup
    let res = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    // Indicatif setup
    let pb: ProgressBar = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    pb.set_message(&format!("Downloading {}", url));

    // download chunks
    let mut file = File::create(path).or(Err(format!("Failed to create file '{}'", path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write_all(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(&format!("Downloaded {} to {}", url, path));
    Ok(())
}
