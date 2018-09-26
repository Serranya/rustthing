extern crate bencode;

use bencode::BencodeValue;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::{env, process};

// Maximal allowed size for .torrent files in bytes
// const MAX_FILE_SIZE: i32 = 1024 * 1024;
#[derive(Debug)]
struct Metainfo {
	announce: String,
	name: String,
	piece_length: i64, // should be unsigned
	pieces_hash: Vec<u8>,
	length: Option<i64>
}

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
			Ok(BencodeValue::EndOfFile) => break,
			Ok(val) => println!("{:#?}", bencode_to_metainfo(val).unwrap()),
			Err(ref err) => {
				println!("{}", err);
				return 1;
			}
		}
	}

	0
}

fn bencode_to_metainfo(val: BencodeValue) -> Result<Metainfo, String> {
	let mut dict = if let BencodeValue::Dictionary(d) = val {
		d
	} else {
		return Err(String::from("val must be of type Dictionary"));
	};

	let announce = dict
		.remove(&String::from("announce").into_bytes())
		.ok_or_else(|| "Missing announce element")?;
	let announce = if let BencodeValue::String(announce) = announce {
		announce
	} else {
		return Err(String::from("Announce must be String"));
	};
	let announce = String::from_utf8_lossy(&announce);


	let info = dict
		.remove(&String::from("info").into_bytes())
		.ok_or_else(|| "Missing info element")?;
	let mut info = if let BencodeValue::Dictionary(info) = info {
		info
	} else {
		return Err(String::from("val must be of type Dictionary"));
	};

	let name = info.remove(&String::from("name").into_bytes()).ok_or_else(|| "Missing name element")?;
	let name = if let BencodeValue::String(name) = name {
		name
	} else {
		return Err(String::from("name must be String"));
	};
	let name = String::from_utf8_lossy(&name);

	let piece_length = info.remove(&String::from("piece length").into_bytes()).ok_or_else(|| "Missing name element")?;
	let piece_length = if let BencodeValue::Integer(piece_length) = piece_length {
		piece_length
	} else {
		return Err(String::from("name must be String"));
	};

	let pieces = info.remove(&String::from("pieces").into_bytes()).ok_or_else(|| "Missing name element")?;
	let pieces_hash = if let BencodeValue::String(pieces) = pieces {
		pieces
	} else {
		return Err(String::from("pieces must be String"));
	};

	let length = info.remove(&String::from("length").into_bytes());
	let length = if length.is_some() {
		if let BencodeValue::Integer(length) = length.unwrap() {
			Option::Some(length)
		} else {
			return Err(String::from("length must be Integer"));
		}
	} else {
		Option::None
	};

	if length.is_none() {
		let _files = info.remove(&String::from("files").into_bytes());
	}

	Ok(Metainfo {
		announce: announce.to_string(),
		name: name.to_string(),
		piece_length,
		pieces_hash,
		length
	})
}
