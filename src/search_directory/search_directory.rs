use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

pub async fn search_directory(
    directories: &Arc<Mutex<VecDeque<PathBuf>>>,
    pending_files: &Arc<Mutex<Vec<PathBuf>>>,
    verbose: bool,
) -> anyhow::Result<()> {
    let mut directories_guard = directories.lock().await;
    let next_directory = directories_guard.get(0);
    match next_directory {
        None => {}
        Some(directory) => {
            let entries = fs::read_dir(directory).await;
            let mut entries = match entries {
                Ok(entries) => entries,
                Err(e) => {
                    if verbose {
                        println!("search_directory read_dir error: {:#?}", e);
                    }
                    directories_guard.pop_front();
                    return Err(e.into());
                }
            };
            let mut tmp_dirs: Vec<PathBuf> = Vec::new();
            let mut tmp_files: Vec<PathBuf> = Vec::new();
            loop {
                let entry = entries.next_entry().await;
                let entry = match entry {
                    Err(e) => {
                        if verbose {
                            println!("search_directory next_entry error: {:#?}", e);
                        }
                        break;
                    }
                    Ok(entry) => entry,
                };
                let entry = match entry {
                    None => break,
                    Some(entry) => entry,
                };
                let file_type = entry.file_type().await;
                let file_type = match file_type {
                    Err(e) => {
                        if verbose {
                            println!("search_directory file_type error: {:#?}", e);
                        }
                        break;
                    }
                    Ok(file_type) => file_type,
                };
                if file_type.is_dir() {
                    tmp_dirs.push(entry.path());
                } else if file_type.is_file() {
                    tmp_files.push(entry.path());
                }
            }
            directories_guard.extend(tmp_dirs);
            pending_files.lock().await.extend(tmp_files);
        }
    }
    directories_guard.pop_front();
    Ok(())
}
