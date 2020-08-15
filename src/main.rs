extern crate fchat3_log_lib;
#[macro_use(load_yaml)]
extern crate clap;
extern crate handlebars;
#[macro_use(Serialize)]
extern crate serde;
extern crate chrono;
extern crate regex;
use clap::App;
use fchat3_log_lib::{FChatMessageReader, FChatMessageReaderReversed};
use std::fs;
use std::path::{PathBuf};
use std::io::BufReader;
mod consumers;
use consumers::FChatLogConsumer;
use consumers::LogConsumer;

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
    let mut app = App::from_yaml(yml);
    let matches = app.clone().get_matches();
    let mut files_to_process = Vec::<PathBuf>::new();
    let recursive = matches.is_present("recursive");
    let reverse_read = matches.is_present("reverse");

    let consumer: Option<Box<dyn FChatLogConsumer>>;
    if matches.is_present("html") {
        if !matches.is_present("output") {
            eprintln!("HTML output requires a place to output.");
            app.print_help().unwrap();
            return
        }
        let output = matches.value_of("output").unwrap();
        let mut html_consumer = consumers::HTMLConsumer::new();
        if html_consumer.configure(None, PathBuf::from(output)).is_err() {
            return
        }
        consumer = Some(Box::new(html_consumer));
    } else {
        consumer = Some(Box::new(consumers::StdoutConsumer::new()));
    }
    if consumer.is_none() {
        panic!("consumer is empty and it should not be.\nThis is a bug.")
    }
    let consumer = consumer.unwrap();

    match matches.values_of("files") {
        Some(files) => {
            for file in files {
                collect_files(&mut files_to_process, PathBuf::new().join(file), recursive) 
            }
        }
        None => {}
    }
    for file in files_to_process {
        let mut character_name = "Unknown";
        match file.parent() {
            Some(path) => {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                if file_name == "logs" {
                    match path.parent() {
                        Some(path) => {
                            character_name = path.file_name().unwrap().to_str().unwrap();
                        },
                        None => {}
                    }
                }
            },
            None => {}
        }

        let fd = BufReader::new(fs::File::open(file.to_owned()).unwrap());
        eprintln!("Reading {:?}", file.to_str().unwrap());
        let log_name = file.file_name().unwrap().to_str().unwrap();
        if reverse_read {
            let mut reader = FChatMessageReaderReversed::new(fd);
            loop {
                match consumer.consume(reader.next(), log_name, Some(character_name)) {
                    Ok(true) => { continue }
                    Ok(false) => { break }
                    Err(err) => { eprintln!("Problem consuming:\n{:?}", err); break}
                }
            }
        } else {
            let mut reader = FChatMessageReader::new(fd);
            loop {
                match consumer.consume(reader.next(), log_name, Some(character_name)) {
                    Ok(true) => { continue }
                    Ok(false) => { break }
                    Err(err) => { eprintln!("Problem consuming:\n{:?}", err); break}
                }
            }
        }
    }
    eprintln!("Finished.")
}
