mod search_directory;
mod search_file;

use clap::Parser;
use futures::future::join_all;
use regex::Regex;
use search_directory::*;
use search_file::*;
use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "simple grep")]
#[command(author = "Ohad Dahan <ohaddahan@gmail.com>")]
#[command(version = "0.1.0")]
#[command(about = "Does awesome things", long_about = None)]
struct Cli {
    #[arg(index = 1)]
    text: String,
    #[arg(index = 2)]
    path: Option<String>,
    /// Sets if it's a regex or not, if flag is present it's on
    #[arg(short, long, default_value = "false")]
    regex: bool,
    /// Sets the level of verbosity, if flag is present it's on
    #[arg(short, long, default_value = "false")]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let pending_files: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    let results: Arc<Mutex<Vec<FileResult>>> = Arc::new(Mutex::new(Vec::new()));
    let directories: Arc<Mutex<VecDeque<PathBuf>>> = Arc::new(Mutex::new(VecDeque::new()));
    let path = cli
        .path
        .map_or(PathBuf::from(env::current_dir()?), PathBuf::from);
    let verbose = cli.verbose;
    directories.lock().await.push_back(path);

    let regex = Arc::new(Mutex::new(match cli.regex {
        true => Regex::new(&cli.text).unwrap(),
        false => Regex::new(&*format!(".*{}.*", &cli.text)).unwrap(),
    }));

    if verbose {
        println!("regex: {:#?}", regex.lock().await.as_str());
    }

    let directories2 = directories.clone();
    let pending_files2 = pending_files.clone();
    let search_in_directories = tokio::spawn(async move {
        loop {
            match search_directory(&directories2, &pending_files2, verbose).await {
                Ok(_) => {}
                Err(e) => {
                    println!("search_directory error: {:#?}", e);
                }
            }
            if directories2.lock().await.is_empty() {
                break;
            }
        }
    });

    let directories3 = directories.clone();
    let pending_files3 = pending_files.clone();
    let results2 = results.clone();
    let process_files = tokio::spawn(async move {
        loop {
            match search_file(&pending_files3, &results2, &regex, verbose).await {
                Ok(_) => {}
                Err(e) => {
                    println!("search_file error: {:#?}", e);
                }
            }
            if directories3.lock().await.is_empty() && pending_files3.lock().await.is_empty() {
                break;
            }
        }
    });
    let tasks = vec![search_in_directories, process_files];
    join_all(tasks).await;
    let results = results.lock().await;
    for result in results.iter() {
        println!("{:#?}:", result.path);
        for line in &result.lines {
            println!("{}:{}", line.0, line.1);
        }
    }
    Ok(())
}
