use anyhow::{Context, Ok, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use nix_daemon::{Progress, Store};
use reqwest::{self, Client};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
// use std::cmp::min;
// use std::io::prelude::*;
use std::process::Stdio;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // let flake_url = "github:msftyago/nix#nixosConfigurations.yago.config.system.build.toplevel";
    let flake_url = "nixpkgs#hello";
    let client = Client::new();

    let child = tokio::process::Command::new("nix")
        .args(["build", flake_url, "--dry-run"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to take child")?;
    // println!("Child: {child:?}");
    let out = child
        .wait_with_output()
        .await
        .context("Failed to take output")?;

    let (fetched, _built, _) = String::try_from(out.stderr)
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .fold((Vec::new(), Vec::new(), false), parse_dry_lines);

    // TODO: does this while in xinux or its installation
    let mut s = nix_daemon::nix::DaemonStore::builder()
        .connect_unix("/nix/var/nix/daemon-socket/socket")
        .await?;

    // https://docs.rs/nix-daemon/latest/nix_daemon/
    let total_download: u64 = s
        .query_missing(&fetched)
        .result()
        .await
        .into_iter()
        .map(|i| i.download_size)
        .sum();

    // Indicatif setup
    let pb: ProgressBar = ProgressBar::new(total_download);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    let pb = Arc::new(pb);

    println!("Daemon Store: {total_download:?}");
    // todo!();
    for fetch in fetched.iter() {
        // let nar = s.query_pathinfo(&fetch).result().await.unwrap().into_iter().map(|x| x.nar_hash);
        // let nar = s.query_pathinfo(&fetch).result().await?;

        let file_name = fetch.split("-").nth(1).context("context")?;
        let path = format!(
            "/home/letrec/workspace/self/progress-rs/tmp/tmp_{}",
            file_name
        );
        download(&client, fetch, &path, Arc::clone(&pb))
            .await
            .context("Failed to donwload in for")?;
    }
    pb.finish_with_message("All packages downloaded");
    Ok(())
}

fn parse_nar_hash(line: &str) -> Option<&str> {
    line.split("/").nth(3)?.split("-").next()
}

fn parse_dry_lines(
    (mut fetched, mut built, is_fetched): (Vec<String>, Vec<String>, bool),
    line: String,
) -> (Vec<String>, Vec<String>, bool) {
    // println!("LINE: {line}");
    if line.contains("will be fetched") {
        return (fetched, built, true);
    }

    if line.contains("will be built") {
        return (fetched, built, false);
    }

    if let Some(hash) = parse_nar_hash(&line) {
        // println!("Hash: {hash}");
        if is_fetched {
            // fetched.push(hash.to_string());
            fetched.push(line.trim().to_string());
        } else {
            built.push(line.trim().to_string());
        }
    }

    (fetched, built, is_fetched)
}

enum DownloadProgress {
    Downloaded,
    Downloading,
    Error,
}

pub async fn download(client: &Client, nar: &str, path: &str, pb: Arc<ProgressBar>) -> Result<()> {
    let cache_xinux = format!("https://cache.xinux.uz/{nar}.narinfo");
    let cache_nixos = format!("https://cache.nixos.org/{nar}.narinfo");

    // Reqwest setup
    let mut res = client
        .get(&cache_xinux)
        .send()
        .await
        .context(format!("Failed to GET from '{}'", &cache_xinux))?;
    if !res.status().is_success() {
        res = client
            .get(&cache_nixos)
            .send()
            .await
            .context(format!("Failed to GET from '{}'", &cache_nixos))?;
        println!("RES in nixos: {res:?}");
    }

    println!("RES: {res:?}");
    pb.set_message(&format!(
        "Downloading {}",
        nar.split('/').last().context("Can not take nar")?
    ));
    // let size = res
    //     .content_length()
    //     .context(format!("Failed to get content length from '{}'", &url))?;

    // download chunks
    let mut file = File::create(path).await.context(format!("Failed to create file '{}'", path))?;
    // let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.context(format!("Error while downloading file"))?;
        file.write_all(&chunk)
            .await
            .context(format!("Error while writing to file"))?;
        // let new = min(downloaded + (chunk.len() as u64), size);
        // downloaded = new;
        pb.inc(chunk.len() as u64);
    }

    Ok(())
}
