extern crate fchat3_log_lib;
#[macro_use(load_yaml)]
extern crate clap;
use clap::App;
use fchat3_log_lib::{FChatMessageReader, structs::ParseError};
use fchat3_log_lib::structs::{FChatMessageType};
use std::fs;
use std::path::{PathBuf};
use std::io::BufReader;

fn collect_files(collection: &mut Vec<PathBuf>, path: PathBuf, can_recurse: bool) {
    if !path.exists() {
        eprintln!("{} does not exist", path.to_str().unwrap());
    } else if path.is_dir() {
        if can_recurse {
            for entry in fs::read_dir(path).unwrap() {
                let entry_path = entry.unwrap().path();
                // logs do not have an extension.
                if entry_path.extension() == None {
                    collect_files(collection, entry_path, can_recurse)
                }
            }
        } else {
            eprint!("{:?} is a directory, and recursion is not enabled", path)
        }
    } else {
        collection.push(path)
    }
}

fn main() {
    let yml = load_yaml!("app.yaml");
    let app = App::from_yaml(yml);
    let matches = app.get_matches();
    let mut files_to_process = Vec::<PathBuf>::new();
    match matches.values_of("files") {
        Some(files) => {
            for file in files {
                collect_files(&mut files_to_process, PathBuf::new().join(file), matches.is_present("recursive"))
            }
        }
        None => {}
    }
    for file in files_to_process {
        let fd = BufReader::new(fs::File::open(file.to_owned()).unwrap());
        let reader = FChatMessageReader::new(fd);
        eprintln!("Reading {:?}", file.to_str().unwrap());
        for result in reader {
            match result {
                Ok(message) => {
                    let body = message.body;
                    let datetime = message.datetime;
                    let sender = message.sender;
                    print!("[{}] [{}] ", file.file_name().unwrap().to_str().unwrap(), datetime);
                    match body {
                        FChatMessageType::Message(string) => { println!("{}: {}", sender, string) }
                        FChatMessageType::Action(string) => { println!("{} {}", sender, string) }
                        FChatMessageType::Ad(string) => { println!("[AD] {}: {}", sender, string) }
                        FChatMessageType::Roll(string) => { println!("{} {}", sender, string) }
                        FChatMessageType::Warn(string) => { println!("[!WARN!] {}: {}", sender, string) }
                        FChatMessageType::Event(string) => { println!("[!EVENT!] {}: {}", sender, string) }
                    }
                }
                Err(err) => {
                    match err {
                        ParseError::EOF(_) => {
                            //eprintln!("Reached end of file!");
                        }
                        _ => {
                            eprintln!("{:?}", err);
                        }
                    }
                    break;
                }
            }
        }
    }
    println!("Hello World")
}
