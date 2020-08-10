extern crate fchat3_log_lib;
#[macro_use(load_yaml)]
extern crate clap;
use clap::App;
use fchat3_log_lib::{FChatMessageReader, FChatMessageReaderReversed, structs::ParseError};
use fchat3_log_lib::structs::{FChatMessageType};
use std::fs;
use std::path::{PathBuf};
use std::io::BufReader;
mod consumers;
use consumers::FChatLogConsumer;

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
    let recursive = matches.is_present("recursive");
    let reverse_read = matches.is_present("reverse");
    match matches.values_of("files") {
        Some(files) => {
            for file in files {
                collect_files(&mut files_to_process, PathBuf::new().join(file), recursive) 
            }
        }
        None => {}
    }
    for file in files_to_process {
        let fd = BufReader::new(fs::File::open(file.to_owned()).unwrap());
        eprintln!("Reading {:?}", file.to_str().unwrap());
        let log_name = file.file_name().unwrap().to_str().unwrap();
        if reverse_read {
            let reader = FChatMessageReaderReversed::new(fd);
            for result in reader {
                match consumers::StdoutConsumer::consume(result, log_name, None) {
                    true => {continue},
                    false => {break}
                }
            }
        } else {
            let reader = FChatMessageReader::new(fd);
            for result in reader {
                match consumers::StdoutConsumer::consume(result, log_name, None) {
                    true => {continue},
                    false => {break}
                }
            }
        }
    }
    eprintln!("Finished.")
}
