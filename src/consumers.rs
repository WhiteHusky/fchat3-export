use handlebars::{Handlebars, HelperDef, RenderContext, Helper, Context, HelperResult, Output};
use fchat3_log_lib::fchat_message::{FChatMessage, FChatMessageType, FChatMessageReaderResult};
use fchat3_log_lib::error::Error;
use std::cell::RefCell;
use chrono::NaiveDate;
use chrono::Datelike;
use std::path::{PathBuf};
use regex::Regex;

pub trait LogConsumer {
    fn new() -> Self;
}

pub trait FChatLogConsumer {
    fn consume(&self, result: Option<FChatMessageReaderResult>, log_name: &str, character_name: Option<&str>) -> bool;
}

// TODO: Probably should be replaced with something less...egregious.
fn get_message(result: Option<FChatMessageReaderResult>) -> Option<FChatMessage> {
    // Thanks to @12Boti#0628 for showing that this is a thing
    match result {
        Some(Ok(message)) => Some(message),
        Some(Err(Error::EOF(_))) => None,
        Some(Err(err)) => { eprintln!("{:?}", err); None},
        None => None
    }
}

pub struct StdoutConsumer {}

impl LogConsumer for StdoutConsumer {
    fn new() -> Self {
        Self {}
    }
}

impl FChatLogConsumer for StdoutConsumer {
    fn consume(&self, result: Option<FChatMessageReaderResult>, log_name: &str, character_name: Option<&str>) -> bool {
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

#[derive(Clone, Copy)]
struct BreaklinesHandlebars {}

impl HelperDef for BreaklinesHandlebars {
    fn call<'reg: 'rc, 'rc>(
            &self,
            h: &Helper<'reg, 'rc>,
            r: &'reg Handlebars<'reg>,
            ctx: &'rc Context,
            rc: &mut RenderContext<'reg, 'rc>,
            out: &mut dyn Output,
        ) -> HelperResult {
            let string = h.param(0).unwrap().value().as_str().unwrap();
            let string = handlebars::html_escape(string);
            let string = Regex::new(r"(\r\n|\n|\r)").unwrap().replace_all(string.as_str(), "</br>").to_string();
            out.write(string.as_str())?;
            Ok(())
    }
}


// RefCell is NOT thread safe! https://doc.rust-lang.org/std/cell/index.html
#[derive(Serialize, Debug)]
struct HTMLConsumerLog<'log> {
    character_name: String,
    log_name: String,
    #[serde(skip_serializing)]
    date_check: Option<NaiveDate>,
    date: String,
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
    logs: RefCell<Vec<HTMLConsumerLog<'html_consumer>>>,
    configured: bool,
    save_location: Option<PathBuf>,
    handlebars_engine: Box<Handlebars<'html_consumer>>
}

impl HTMLConsumer<'_> {
    pub fn configure(&mut self, template: Option<&str>, save_location: PathBuf) -> Result<(), ()> {
        self.handlebars_engine.register_template_string("log", template.unwrap_or(include_str!("./templates/html/log.hbs"))).unwrap();
        self.handlebars_engine.register_helper("breakline", Box::new(BreaklinesHandlebars {}));
        if !save_location.exists() {
            eprint!("{:?} does not exist!", save_location);
            return Err(());
        } else if !save_location.is_dir() {
            eprint!("{:?} is not a directory!", save_location);
            return Err(());
        }
        self.save_location = Some(save_location);
        self.configured = true;
        Ok(())
    }
    fn write_log(&self, log: &mut HTMLConsumerLog) {
        let save_location = self.save_location.as_ref().unwrap();
        let save_location = save_location.join(format!("{}/{}/{}.html", log.character_name, log.log_name, log.date));
        std::fs::create_dir_all(save_location.parent().unwrap()).unwrap();
        let mut file_options = std::fs::OpenOptions::new();
        let file = file_options.create(true).write(true).open(save_location).unwrap();
        self.handlebars_engine.render_to_write("log", &log, file).unwrap(); //std::io::stdout()
    }
}

impl LogConsumer for HTMLConsumer<'_> {
    fn new() -> Self {
        let engine = Handlebars::new();
        Self {
            configured: false,
            handlebars_engine: Box::new(engine),
            logs: RefCell::new(Vec::new()),
            save_location: None
        }
    }
}

impl FChatLogConsumer for HTMLConsumer<'_> {
    fn consume(&self, result: Option<FChatMessageReaderResult>, log_name: &str, character_name: Option<&str>) -> bool {
        if !self.configured{
            panic!("HTMLConsumer needs to be configured first!")
        }
        let message_retrieved = get_message(result);
        let character_name = character_name.unwrap_or("Unknown");
        let mut logs = self.logs.borrow_mut();
        let mut log: Option<&mut HTMLConsumerLog> = None;
        let mut log_index: Option<usize> = None;
        let mut log_found =  false;
        for (index, log_at) in logs.iter_mut().enumerate() {
            if log_at.character_name == character_name && log_at.log_name == log_name {
                log = Some(log_at);
                log_index = Some(index);
                log_found = true;
                break;
            }
        }
        if !log_found {
            logs.push(HTMLConsumerLog {
                character_name: character_name.to_owned(),
                log_name: log_name.to_owned(),
                entries: RefCell::new(Vec::new()),
                date_check: None,
                date: String::new()
            });
            log_index = Some(logs.len() - 1);
            log = Some(logs.last_mut().unwrap());
        }
        let log = log.unwrap();
        let log_index = log_index.unwrap();
        match message_retrieved {
            Some(message) => {
                if !log_found {
                    log.date.push_str(&NaiveDate::from_ymd(message.datetime.year(), message.datetime.month(), message.datetime.day()).to_string());
                    log.date_check = Some(message.datetime.date());
                }
                let m_datetime = &message.datetime;
                let l_datetime = &log.date_check.unwrap();
                
                // If the date is different, render now and drain the entries.
                if m_datetime.year() != l_datetime.year() || m_datetime.month() != l_datetime.month() || m_datetime.day() != l_datetime.day() {
                    eprintln!("Writing {} {} {}", log.character_name, log.log_name, log.date);
                    self.write_log(log);
                    log.entries = RefCell::new(Vec::new());
                    log.date = NaiveDate::from_ymd(message.datetime.year(), message.datetime.month(), message.datetime.day()).to_string();
                    log.date_check = Some(message.datetime.date());
                }

                let sender_name = message.sender;
                let datetime = message.datetime.to_string();
                let message_body_class_hints: &str;
                let message_body: String;

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

                log.entries.borrow_mut().push(HTMLConsumerLogEntry {
                    sender_name,
                    datetime,
                    message_body,
                    message_body_class_hints
                });
            }
            None => {
                if log.entries.borrow().len() > 0 {
                    eprintln!("Writing {} {} {}", log.character_name, log.log_name, log.date);
                    self.write_log(log);
                }
                logs.remove(log_index);
                return false;
            }
        }
        true
    }
}