extern crate bytes;
extern crate byteorder;
extern crate chrono;
extern crate clap;
extern crate glob;
extern crate rayon;
use std::{str, usize};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind, BufReader, BufWriter, BufRead, SeekFrom, Read, Seek};
use std::path::{PathBuf};
use std::borrow::{Cow};
use byteorder::{ReadBytesExt, LittleEndian};
use chrono::{NaiveDateTime};
use clap::{Arg, App};
use glob::glob;
use std::sync::atomic::{Ordering, AtomicBool, ATOMIC_BOOL_INIT};
static THREAD_PANIC: AtomicBool = ATOMIC_BOOL_INIT;
static THREAD_ERROR: AtomicBool = ATOMIC_BOOL_INIT;

struct FchatMessage {
    epoch_seconds: u32,
    message_type: u8,
    sender: String,
    text: String,
    bytes_used: usize
}

impl Default for FchatMessage {
    fn default() -> FchatMessage {
        FchatMessage {
            epoch_seconds: 0,
            message_type: 0,
            sender: String::new(),
            text: String::new(),
            bytes_used: 0
        }
    }
}

fn main() -> Result<(), Error> {
    let matches = App::new("F-Chat 3.0 Export Program")
                    .version("0.1.0")
                    .author("Carlen White <whitersuburban@gmail.com>")
                    .about("Reads F-Chat 3.0 data files and exports them to the stout or as individual files (NOT IMPLEMENTED). Able to read data files from F-Chat 3.0.10.")
                    .arg(Arg::with_name("file")
                        .short("f")
                        .long("file")
                        .value_name("FILE")
                        .help("Which file or files to read from.")
                        .takes_value(true)
                        .multiple(true)
                        .required(true))
                    /*.arg(Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .value_name("OUTPUT")
                        .help("Location to output to. Location is assumed a folder unless with the single switch. Will not overwrite unless given the cobble switch.")
                        .takes_value(true)
                        .conflicts_with("directory"))
                    .arg(Arg::with_name("single")
                        .long("single")
                        .help("Assume location is a file and write output to it.")
                        .conflicts_with("output"))
                    .arg(Arg::with_name("cobble")
                        .short("c")
                        .long("cobble")
                        .help("Allow files to be overwritten."))*/
                    .arg(Arg::with_name("recursive")
                        .short("r")
                        .long("recursive")
                        .help("Allow moving recursively through directories."))
                    .arg(Arg::with_name("allow-hidden")
                        .short("h")
                        .long("allow-hidden")
                        .help("Allow reading and traversal of hidden files or folders."))
                    .arg(Arg::with_name("ignore-check")
                        .short("i")
                        .long("ignore-check")
                        .help("Disables message length checking. Can cause unstable behavior."))
                    .arg(Arg::with_name("reverse")
                        .short("z")
                        .long("reverse")
                        .help("Read files from end to start."))
                    .arg(Arg::with_name("single-thread")
                        .short("s")
                        .long("single-thread")
                        .help("Do not use parallel reading. Read files one at a time."))
                    /*.arg(Arg::with_name("verbose")
                        .short("v")
                        .long("verbose")
                        .help("Show messages to stout, regardless if a output file has been selected."))*/
                    .arg(Arg::with_name("dry-run")
                        .short("d")
                        .long("dry-run")
                        .help("Checks files, but does not run."))
    .get_matches();
    let mut file_list: Vec<PathBuf> = Vec::new();
    let can_recurse = matches.is_present("recursive");
    let can_read_hidden = matches.is_present("allow-hidden");
    let ignore_check = matches.is_present("ignore-check");
    let read_reverse = matches.is_present("reverse");
    let single_thread = matches.is_present("single-thread");
    let dry_run = matches.is_present("dry-run");
    let verbose = matches.is_present("verbose");
    let single_file = matches.is_present("single");
    let allow_cobble = matches.is_present("cobble");
    let output_location = match matches.value_of("output") {
        Some(out) => out,
        None => ""
    };
    let mut output_path = PathBuf::new();
    if output_location != "" {
        output_path.push(output_location);
        println!("Outputting to {}", output_path.to_string_lossy());
        //if !single_file && output_path.extension().is_some() { return Err(Error::new(ErrorKind::InvalidInput, "You need to have the single-file switch to write to a file."))}
        match output_path.exists() {
            true => {
                if single_file {
                    if output_path.is_dir() { return Err(Error::new(ErrorKind::AlreadyExists, "Can not create file as a directory with the same name already exists.")) }
                } else {
                    if output_path.is_file() { return Err(Error::new(ErrorKind::AlreadyExists, "Can not create directory as a file with the same name already exists.")) }
                }
            },
            false => {
                match output_path.parent() {
                    Some(parent_path) => {
                        fs::create_dir_all(parent_path)?;
                    },
                    None => { return Err(Error::new(ErrorKind::NotFound, "Can not find parent to create folders."))}
                }
            }
        }
    }
    match matches.values_of("file") {
        Some(files) => {
            for file_argument in files {
                match glob(file_argument) {
                    Ok(paths_collected) => {
                        for entry in paths_collected {
                            match entry {
                                Ok(path) => {
                                    match check_and_add_files(&mut file_list, path, can_read_hidden, can_recurse) {
                                        Ok(_) => {},
                                        Err(e) => {
                                            println!("Warning: {:?}", e);
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("Warning: {:?}", e);
                                }
                            };
                        }
                    },
                    Err(e) => {
                        //println!("Problem when collecting files for glob. {:?}", e);
                        return Err(Error::new(ErrorKind::InvalidInput, e));
                    }
                };
                
            }
        }
        None => {}
    };

    if file_list.len() == 0 {
        //println!("No files matched glob. Check switches or provided path.");
        return Err(Error::new(ErrorKind::NotFound, "No files matched glob. Check switches or provided path."));
    }
    if dry_run {
        for file_path in file_list {
            println!("Would read: {:?}", file_path);
        }
        return Ok(());
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(if single_thread || single_file {1} else {0})
        .panic_handler(|panic| {
            println!("Thread Panic!");
            println!("{:?}", panic);
            THREAD_PANIC.store(true, Ordering::Relaxed);
        })
        .build()
        .expect("Could not create threads.");
    
    pool.scope(|s| {
        for file_path in file_list {
            s.spawn(|_| {
                let file_path = file_path; // Reown the value?
                let file_path_absolute = fs::canonicalize(&file_path).expect("Failed to get absolute path.");
                println!("Reading: {:?}", file_path);
                let file = match File::open(&file_path_absolute) {
                    Ok(opened_file) => opened_file,
                    Err(e) => {
                        THREAD_ERROR.store(true, Ordering::Relaxed);
                        println!("Problem opening file {}: {:?}", file_path.to_string_lossy(), e);
                        return;
                    }
                };
                let mut buf_reader = BufReader::new(file);
                if read_reverse {
                    buf_reader.seek(SeekFrom::End(0)).expect("Failed to seek to end of file.");
                }
                let character_name = match file_path_absolute.parent() {
                    Some(path) => {
                        let path_file_name = path.file_name().unwrap().to_string_lossy();
                        if path_file_name == "logs" {
                            match path.parent() {
                                Some(path2) => {
                                    path2.file_name().unwrap().to_string_lossy()
                                },
                                None => {
                                    path_file_name
                                }
                            }
                        } else {
                            path_file_name
                        }
                    },
                    None => {
                        Cow::from(String::from("UNKNOWN"))
                    }
                };
                /*let output_file:fs::File;
                let output_buffer:BufWriter;
                if single_file {
                    if !allow_cobble && output_path.exists() {
                        THREAD_ERROR.store(true, Ordering::Relaxed);
                        println!("{} already exists. Enable the cobble switch to allow overwrite", output_path.to_string_lossy());
                        return;
                    }
                    output_file = match File::create(&output_path) {
                        Ok(opened_file) => opened_file,
                        Err(e) => {
                            THREAD_ERROR.store(true, Ordering::Relaxed);
                            println!("Problem creating file {}: {:?}", output_path.to_string_lossy(), e);
                            return;
                        }
                    };
                    output_file.write("AAA");
                }*/
                loop {
                    let msg = match fchat_deserialize_message(&mut buf_reader, ignore_check, read_reverse) {
                        Ok(message) => message,
                        Err(e) => {
                            THREAD_ERROR.store(true, Ordering::Relaxed);
                            println!("Problem parsing file {}: {:?}", file_path.to_string_lossy(), e);
                            return;
                        }
                    };
                    let timestamp = NaiveDateTime::from_timestamp(msg.epoch_seconds as i64, 0);
                    let colon = match msg.message_type {
                        1 | 5 => "",
                        _ => ": "
                    };
                    println!("[{}] [{}] [{}] {}{}{}", character_name, file_path_absolute.file_stem().unwrap().to_string_lossy(), timestamp.format("%Y-%m-%d %H:%M:%S"), msg.sender, colon, msg.text);
                    if read_reverse {
                        if buf_reader.seek(SeekFrom::Current(0)).unwrap() == 0 {
                            return;
                        }
                    } else if buf_reader.fill_buf().unwrap().len() == 0 {
                        return;
                    }
                }
            });
        }
    });
    if THREAD_ERROR.load(Ordering::Relaxed) {
        return Err(Error::new(ErrorKind::Other, "Some files could not be processed. Check above for messages."));
    }
    if THREAD_PANIC.load(Ordering::Relaxed) {
        return Err(Error::new(ErrorKind::Other, "A thread panicked. Check above for messages."));
    }
    return Ok(());
}

fn check_and_add_files(vec_to_save_to: &mut Vec<PathBuf>, path: PathBuf, can_read_hidden: bool, can_recurse: bool) -> std::io::Result<bool> {
    let md = match fs::metadata(&path) {
        Ok(meta) => meta,
        Err(e) => {
            println!("Warning: \"{:?}\" {:?}", path, e);
            return Ok(false);
        }
    };
    match path.file_stem() {
        Some(filename) => {
            match filename.to_str() {
                Some(str_filename) => {
                    if str_filename == "settings" || (!can_read_hidden && str_filename.chars().next().unwrap_or('.') == '.')  {
                        return Ok(false);
                    }
                },
                None => {
                    return Ok(false);
                }
            };
        },
        None => {
            return Ok(false);
        }
    };
    if md.is_dir() && can_recurse {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            check_and_add_files(vec_to_save_to, entry.path(), can_read_hidden, can_recurse)?;
        }
        return Ok(true);
    } else if md.is_file() && path.extension() == None {
        vec_to_save_to.push(path);
        return Ok(true);
    }
    return Ok(false);
}
/* NOTE FOR FUTURE IDX PARSING:
    [
        name_len:u8,
        name_utf8:utf8_string * name_len,
        days_from_unix_epoch:u16, -- AKA unix_epoch/unix_epoch_seconds_in_day = day_epoch_midnight
        day_offset:u24, -- Offset in associated file
        ...
    ]
*/
fn fchat_deserialize_message<B: Read + Seek + ReadBytesExt>(buffer: &mut B, ignore_check: bool, reverse: bool) -> std::io::Result<FchatMessage> {
    let mut check_digit:u16 = 0;
    if reverse {
        buffer.seek(SeekFrom::Current(-2))?;
        check_digit = buffer.read_u16::<LittleEndian>()?;
        buffer.seek(SeekFrom::Current(-2 - check_digit as i64))?;
    }
    let mut message = FchatMessage::default();
    message.epoch_seconds = buffer.read_u32::<LittleEndian>()?;
    message.message_type = buffer.read_u8()?;
    let sender_length:u8 = buffer.read_u8()?;
    let mut text = vec![0; sender_length as usize];
    buffer.read_exact(text.as_mut_slice())?;
    message.sender.push_str(str::from_utf8(&text).unwrap());
    let text_length:u16 = buffer.read_u16::<LittleEndian>()?;
    text = vec![0; text_length as usize];
    buffer.read_exact(text.as_mut_slice())?;
    message.text.push_str(str::from_utf8(&text).unwrap());
    message.bytes_used = 4 + 1 + 1 + message.sender.capacity() + 2 + message.text.capacity();
    if !reverse {
        check_digit = buffer.read_u16::<LittleEndian>()?;
    } else {
        buffer.seek(SeekFrom::Current((check_digit as i64) * -1))?;
    }
    if !ignore_check && check_digit != message.bytes_used as u16 {
        Err(Error::new(ErrorKind::InvalidData, format!("Size mismatch! Expected {}, instead counted {}.", message.bytes_used, check_digit)))
    } else {
        message.bytes_used += 2;
        Ok(message)
    }
}
