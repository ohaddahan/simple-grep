use regex::Regex;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct FileResult {
    pub(crate) path: PathBuf,
    pub(crate) lines: Vec<(usize, String)>,
}

pub async fn search_file(
    pending_files: &Arc<Mutex<Vec<PathBuf>>>,
    results: &Arc<Mutex<Vec<FileResult>>>,
    regex: &Arc<Mutex<Regex>>,
    verbose: bool,
) -> anyhow::Result<()> {
    return match pending_files.lock().await.pop() {
        None => Ok(()),
        Some(file_name) => {
            let re = regex.lock().await.clone();
            let file = File::open(file_name.clone()).await;
            let file = match file {
                Err(e) => {
                    if verbose {
                        println!("search_file open error: {:#?}", e);
                    }
                    return Err(e.into());
                }
                Ok(file) => file,
            };
            let reader = BufReader::new(file);
            let mut lines = reader.lines();
            let mut line_number = 0;
            let mut file_result = FileResult {
                path: file_name,
                lines: Vec::new(),
            };
            while let Some(line) = lines.next_line().await? {
                if re.is_match(&line) {
                    file_result.lines.push((line_number, line.to_string()));
                }
                line_number += 1;
            }
            if file_result.lines.len() > 0 {
                results.lock().await.push(file_result);
            }
            Ok(())
        }
    };
}
