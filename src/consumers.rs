use handlebars::Handlebars;
use fchat3_log_lib::structs::{FChatMessageType, ParseError, ReaderResult, FChatMessage};
use std::cell::RefCell;

pub trait LogConsumer {
    fn new(log_name: &str, character_name: Option<&str>) -> Self;
}

pub trait FChatLogConsumer {
    fn consume(&self, result: Option<ReaderResult>, log_name: &str, character_name: Option<&str>) -> bool;
}

// TODO: Probably should be replaced with something less...egregious.
fn get_message(result: Option<ReaderResult>) -> Option<FChatMessage> {
    // Thanks to @12Boti#0628 for showing that this is a thing
    match result {
        Some(Ok(message)) => Some(message),
        Some(Err(ParseError::EOF(_))) => None,
        Some(Err(err)) => { eprintln!("{:?}", err); None},
        None => None
    }
}

pub struct StdoutConsumer {}

impl LogConsumer for StdoutConsumer {
    fn new(log_name: &str, character_name: Option<&str>) -> Self {
        Self {}
    }
}

impl FChatLogConsumer for StdoutConsumer {
    fn consume(&self, result: Option<ReaderResult>, log_name: &str, character_name: Option<&str>) -> bool {
        let message_retrieved = get_message(result);
        match message_retrieved {
            Some(message) => {
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
            None => {return false}
        }
        true
    }
}

// RefCell is NOT thread safe! https://doc.rust-lang.org/std/cell/index.html
#[derive(Serialize, Debug)]
struct HTMLConsumerLog<'log> {
    character_name: String,
    log_name: String,
    entries: RefCell<Vec<HTMLConsumerLogEntry<'log>>>
}

#[derive(Serialize, Debug)]
struct HTMLConsumerLogEntry<'entry> {
    datetime: String,
    sender_name: String,
    message_body_class_hints: &'entry str,
    message_body: String
}
pub struct HTMLConsumer<'html_consumer> {
    log: HTMLConsumerLog<'html_consumer>,
    configured: bool,
    handlebars_engine: Box<Handlebars<'html_consumer>>
}

impl HTMLConsumer<'_> {
    pub fn configure(&mut self, template: Option<&str>) {
        self.handlebars_engine.register_template_string("log", template.unwrap_or(include_str!("./templates/html/log.hbs"))).unwrap();
        self.configured = true;
    }
}

impl LogConsumer for HTMLConsumer<'_> {
    fn new(log_name: &str, character_name: Option<&str>) -> Self {
        let mut engine = Handlebars::new();
        let refed_character_name = character_name.unwrap_or("Unknown").to_owned();
        let refed_log_name = log_name.to_owned();
        Self {
            configured: false,
            handlebars_engine: Box::new(engine),
            log: HTMLConsumerLog{
                character_name: refed_character_name,
                log_name: refed_log_name,
                entries: RefCell::new(Vec::new()),
            },
        }
    }
}

impl FChatLogConsumer for HTMLConsumer<'_> {
    fn consume(&self, result: Option<ReaderResult>, log_name: &str, character_name: Option<&str>) -> bool {
        if !self.configured{
            panic!("HTMLConsumer needs to be configured first!")
        }
        let message_retrieved = get_message(result);
        match message_retrieved {
            Some(message) => {
                let sender_name = message.sender;
                let datetime = message.datetime.to_string();
                let mut message_body_class_hints: &str;
                let mut message_body: String;

                match message.body {
                    FChatMessageType::Message(string) => {
                        message_body_class_hints = "message";
                        message_body = string;
                    }
                    FChatMessageType::Action(string) => {
                        message_body_class_hints = "action";
                        message_body = string;
                    }
                    FChatMessageType::Ad(string) => {
                        message_body_class_hints = "ad";
                        message_body = string;
                    }
                    FChatMessageType::Roll(string) => {
                        message_body_class_hints = "roll";
                        message_body = string;
                    }
                    FChatMessageType::Warn(string) => {
                        message_body_class_hints = "warn";
                        message_body = string;
                    }
                    FChatMessageType::Event(string) => {
                        message_body_class_hints = "event";
                        message_body = string;
                    }
                }

                self.log.entries.borrow_mut().push(HTMLConsumerLogEntry {
                    sender_name,
                    datetime,
                    message_body,
                    message_body_class_hints
                });
            }
            None => {
                self.handlebars_engine.render_to_write("log", &self.log, std::io::stdout()).unwrap();
                return false;
            }
        }
        true
    }
}