use std::error::Error;

use clap::{Args, Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::fs;

/**
 * cside create dirname
 * cside update direname
 * cside update dirname --no-discard
 *
 */

const URL: &str = "https://my-json-server.typicode.com/amitbikram/file-api/db";
const EXT: &str = ".txt";
const SEPARATOR: &str = "_";

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Margs {
    #[clap(subcommand)]
    action_type: ActionType,
}

#[derive(Debug, Subcommand)]
enum ActionType {
    Create(CreateCommand),
    Update(UpdateCommand),
}

#[derive(Debug, Args)]
struct CreateCommand {
    dirname: String,
}

#[derive(Debug, Args)]
struct UpdateCommand {
    dirname: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TokenConfig {
    data: Vec<String>,
}

struct FileStatus {
    local_files: Vec<String>,
    remote_files: Vec<String>,
    local_processed_files: Vec<bool>,
    remote_processed_files: Vec<bool>,
}

impl FileStatus {
    fn new() -> Self {
        Self {
            local_files: Vec::new(),
            remote_files: Vec::new(),
            local_processed_files: Vec::new(),
            remote_processed_files: Vec::new(),
        }
    }

    fn add_local_processed_files(&mut self) {
        &self.local_processed_files.push(false);
    }

    fn add_remote_processed_files(&mut self) {
        &self.remote_processed_files.push(false);
    }

    fn add_local_file(&mut self, file_name: String) {
        &self.local_files.push(file_name);
    }

    fn add_remote_file(&mut self, file_name: String) {
        &self.remote_files.push(file_name);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let body = reqwest::get(URL).await?.text().await?;
    let deserialized: TokenConfig = serde_json::from_str(&body).unwrap();
    let loop_len = deserialized.data.len();
    let margs = Margs::parse();

    match margs.action_type {
        ActionType::Create(CreateCommand { dirname }) => {
            fs::create_dir(&dirname).await?;
            for i in 0..loop_len {
                let mut accum = String::from("");
                for j in i..loop_len {
                    accum = accum + &deserialized.data[j];
                    let file_name = accum.clone() + EXT;
                    fs::write(String::from(&dirname) + "/" + &file_name, b"Empty").await?;
                    accum = accum + SEPARATOR;
                }
            }
        }
        ActionType::Update(UpdateCommand { dirname }) => {
            let mut file_status = FileStatus::new();
            fs::metadata(&dirname).await?;
            let mut iter = fs::read_dir(&dirname).await?;

            while let Some(entry) = iter.next_entry().await? {
                let file_name = entry.file_name().into_string().unwrap();
                file_status.add_local_file(file_name);
                file_status.add_local_processed_files();
            }

            for i in 0..loop_len {
                let mut accum = String::from("");
                for j in i..loop_len {
                    accum = accum + &deserialized.data[j];
                    let file_name = accum.clone() + EXT;
                    file_status.add_remote_file(file_name);
                    file_status.add_remote_processed_files();
                    accum = accum + SEPARATOR;
                }
            }

            for i in 0..file_status.remote_files.len() {
                for j in 0..file_status.local_files.len() {
                    if file_status.remote_processed_files[i] == true
                        || file_status.local_processed_files[j] == true
                    {
                        //ignore
                        continue;
                    }
                    if file_status.remote_files[i] == file_status.local_files[j] {
                        // if files are same then ignore
                        file_status.remote_processed_files[i] = true;
                        file_status.local_processed_files[j] = true;
                        continue;
                    }

                    if is_same_file(&file_status.remote_files[i], &file_status.local_files[j]) {
                        fs::rename(&file_status.local_files[j], &file_status.remote_files[i])
                            .await?;
                        file_status.remote_processed_files[i] = true;
                        file_status.local_processed_files[j] = true;
                    }
                }
            }

            for i in 0..file_status.remote_files.len() {
                if file_status.remote_processed_files[i] == false {
                    file_status.remote_processed_files[i] = true;
                    let fname = String::from(&dirname) + "/" + &file_status.remote_files[i];
                    fs::write(fname, b"Empty").await?;
                }
            }

            for i in 0..file_status.local_files.len() {
                if file_status.local_processed_files[i] == false {
                    file_status.local_processed_files[i] = true;
                    let fname = String::from(&dirname) + "/" + &file_status.local_files[i];
                    fs::remove_file(fname).await?;
                }
            }
        }
    };

    Ok(())
}

fn is_same_file(remote_file: &str, local_file: &str) -> bool {
    let re = Regex::new(r"[._]").unwrap();
    let mut remote_fields: Vec<&str> = re.split(&remote_file).collect();
    let mut local_fields: Vec<&str> = re.split(&local_file).collect();
    remote_fields.sort();
    local_fields.sort();
    for &r in &remote_fields {
        for &l in &local_fields {
            if l != r {
                return false;
            }
        }
    }

    return true;
}
