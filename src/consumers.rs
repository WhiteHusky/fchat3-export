use fchat3_log_lib::structs::{FChatMessageType, ParseError, ReaderResult};

pub trait FChatLogConsumer {
    fn consume(result: ReaderResult, log_name: &str, character_name: Option<&str>) -> bool;
}

pub struct StdoutConsumer {}

impl FChatLogConsumer for StdoutConsumer {
    fn consume(result: ReaderResult, log_name: &str, character_name: Option<&str>) -> bool {
        match result {
            Ok(message) => {
                let body = message.body;
                let datetime = message.datetime;
                let sender = message.sender;
                let character = character_name.unwrap_or("Unknown");
                print!("[{}] [{}] [{}] ", character, log_name, datetime);
                match body {
                    FChatMessageType::Message(string) => { println!("{}: {}", sender, string) }
                    FChatMessageType::Action(string) => { println!("{}{}", sender, string) }
                    FChatMessageType::Ad(string) => { println!("[AD] {}: {}", sender, string) }
                    FChatMessageType::Roll(string) => { println!("[ROLL] {} {}", sender, string) }
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
                return false
            }
        }
        true
    }
}