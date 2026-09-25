use std::process::Stdio;

use tokio_stream::Stream;

#[tokio::main]
async fn main() {
    let flake_url = "github:msftyago/nix#nixosConfigurations.yago.config.system.build.toplevel";
    // let flake_url = "nixpkgs#go";

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

    // let a = download(&fetched[0]).await;
    // println!("{a:#?}");
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
    Downloading
}

// async fn download(nar: &str) -> impl Stream<Item = DownloadProgress> {
//     let narinfo = reqwest::get(format!("https://cache.xinux.uz/{nar}.narinfo")).await.unwrap().text().await.unwrap();
//     let narinfo = sui_compat::narinfo::NarInfo::parse(&narinfo).unwrap();
//     println!("{narinfo:#?}");
//     let content = reqwest::get(&format!("https://cache.xinux.uz/{}", narinfo.url)).await.unwrap();

//     // content.chunk()
//     tokio_stream::
// }
