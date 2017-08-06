extern crate bencode;

use std::{env, process};
use std::fs::File;
use std::io::Read;
use std::io::BufReader;

// Maximal allowed size for .torrent files in bytes
// const MAX_FILE_SIZE: i32 = 1024 * 1024;

fn main() {
    let exit_code = run_app();
    process::exit(exit_code);
}

fn run_app() -> i32 {
    let mut args = env::args();
    let prog_name = args.next().unwrap();
    if args.len() != 1 {
        println!("Usage: {} FILENAME", prog_name);
        return 1;
    }
    let path = args.next().expect("Missing FILENAME argument");
    let f = match File::open(&path) {
        Ok(file) => file,
        Err(err) => {
            println!("Error while opening {}\n{}", &path, err);
            return 1;
        }
    };

    let mut bytes = BufReader::new(f).bytes();

    loop {
        match bencode::parse_value(&mut bytes) {
            Ok(bencode::BencodeValue::EndOfFile) => break,
            Ok(val) => println!("{:?}", val),
            Err(ref err) => {
                println!("{}", err);
                return 1;
            }
        }
    }

    0
}
