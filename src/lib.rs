use std::collections::HashMap;

use std::io;
use std::io::{Error, ErrorKind};
use std::fmt;

pub enum BencodeValue {
    Integer(i64),
    String(Vec<u8>),
    List(Vec<BencodeValue>),
    Dictionary(HashMap<Vec<u8>, BencodeValue>),
    EndOfFile, // TODO is this needed?
}

enum BencodeValuePrintState<'a> {
    Start,
    List(&'a Vec<BencodeValue>, usize),
    End,
}

impl fmt::Debug for BencodeValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_bencode_value(self, f, 0)
    }
}

fn write_bencode_value(val: &BencodeValue, f: &mut fmt::Formatter, indent: u8) -> fmt::Result {
    /*    let mut state = BencodeValuePrintState::Start;
    let mut current_value = val;

    loop {
        match state {
            BencodeValuePrintState::Start => {
                match *current_value {
                    BencodeValue::Integer(ref val) => {
                        print_integer(f, val)?;
                        state = BencodeValuePrintState::End
                    }
                    BencodeValue::String(ref val) => {
                        print_string(f, val)?;
                        state = BencodeValuePrintState::End
                    }
                    BencodeValue::List(ref val) => {
                        write!(f, "List(")?;
                        state = BencodeValuePrintState::List(val, 0);
                        val.iter();
                    }
                    _ => {
                        panic!("TODO");
                    }
                }
            }
            BencodeValuePrintState::List(ref val, idx) => {
                if let Some(in_val) = val.get(idx) {
                    current_value = in_val;
                } else {
                    write!(f, ")")?;
                    //TODO pop state
                }
            }
            BencodeValuePrintState::End => {
                return Ok(());
            }
        }
    } */

    for _ in 0..indent {
        write!(f, "    ")?;
    }
    match *val {
        BencodeValue::Integer(ref val) => print_integer(f, val),
        BencodeValue::String(ref val) => print_string(f, val),
        BencodeValue::List(ref val) => {
            write!(f, "List(\n")?;
            for elem in val {
                write_bencode_value(elem, f, indent + 1)?;
            }
            write!(f, ")")
        }
        BencodeValue::Dictionary(ref val) => {
            write!(f, "Dictionary(\n")?;
            for (key, value) in val.iter() {
                write!(f, "    ")?;
                print_string(f, key)?;
                write!(f, ":")?;
                write_bencode_value(value, f, indent)?;
                write!(f, "\n")?
            }
            write!(f, ")")
        }
        _ => write!(f, "TODO"),
    }
}

/* impl fmt::Debug for BencodeValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BencodeValue::Integer(ref val) => {
	            write!(f, "Integer({:?})", val)
            }
            BencodeValue::String(ref val) => {
	            write!(f, "String({:?})", String::from_utf8_lossy(val))
            }
            BencodeValue::List(ref val) => {
            	write!(f, "List({:?})", val)
            }
            BencodeValue::Dictionary(ref val) => {
            	write!(f, "Dictionary(\n")?;

            	for (key, value) in val.iter() {
            		write!(f, "\t{:?}:{:?}", key, value)?;
            	}
            	write!(f, "\n)")
            }
            BencodeValue::EndOfFile => {
	            write!(f, "EOF")
            }
        }
    }
}*/

fn print_integer(f: &mut fmt::Formatter, val: &i64) -> fmt::Result {
    write!(f, "Integer({})", val)
}

fn print_string(f: &mut fmt::Formatter, val: &Vec<u8>) -> fmt::Result {
    write!(f, "String({:?})", String::from_utf8_lossy(val))
}

pub fn parse_value(iter: &mut Iterator<Item = io::Result<u8>>) -> io::Result<BencodeValue> {
    let mut iter = iter.peekable();

    loop {
        let byte;
        match iter.peek() {
            Some(result) => {
                match *result {
                    Ok(val) => {
                        byte = val;
                    }
                    Err(ref err) => {
                        println!("Error while reading file {}", err);
                        return Err(Error::new(ErrorKind::Other, "Error while reading file"));
                    }
                }
            }
            _ => break,
        }

        match byte {
            0x30...0x39 => return Ok(BencodeValue::String(parse_string(&mut iter)?)),
            0x64 => return Ok(BencodeValue::Dictionary(parse_dict(&mut iter)?)),
            0x69 => return Ok(BencodeValue::Integer(parse_int(&mut iter)?)),
            0x6c => return Ok(BencodeValue::List(parse_list(&mut iter)?)),
            val => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Unexpected byte {}", val),
                ))
            }
        }
    }

    return Ok(BencodeValue::EndOfFile);
}

fn parse_string(iter: &mut Iterator<Item = io::Result<u8>>) -> io::Result<Vec<u8>> {
    let mut ret = Vec::new();

    loop {
        let curr_byte = iter.next().ok_or(Error::new(
            ErrorKind::InvalidData,
            "File ended while reading string",
        ))??;
        if curr_byte >= 0x30 && curr_byte <= 0x39 {
            ret.push(curr_byte);
        } else if curr_byte == 0x3a {
            break;
        } else {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Expected an integer (byte 0x30 - 0x39) got {:x}",
                    curr_byte
                ),
            ));
        }
    }

    let length = vec_to_int(&ret, false)?;
    let mut ret = Vec::with_capacity(length as usize); //TODO fix potential overflow

    for _ in 0..length {
        let curr_byte = iter.next().ok_or(Error::new(
            ErrorKind::InvalidData,
            "File ended while reading string.",
        ))??;
        ret.push(curr_byte);
    }

    return Ok(ret);
}

fn parse_dict(
    iter: &mut Iterator<Item = io::Result<u8>>,
) -> io::Result<HashMap<Vec<u8>, BencodeValue>> {
    let mut iter = iter.peekable();
    iter.next(); // we don't need the "start of dictionary" indicator

    let mut ret = HashMap::new();

    loop {
        //TODO handle empty dict "de"
        let key = parse_string(&mut iter)?;
        let value = parse_value(&mut iter)?;
        //println!("Adding k:{:?} v:{:?} to dictionary", key, value);
        ret.insert(key, value);

        let test = iter.peek().ok_or(Error::new(
            ErrorKind::InvalidData,
            "File ended while reading dictionary",
        ))?;
        match *test {
            Ok(val) if val == 0x65 => break,
            Ok(_) => {}
            Err(ref err) => {
                println!("Error while reading dictionary {}", err);
                return Err(Error::new(
                    ErrorKind::Other,
                    "Error while reading dictionary",
                ));
            }
        }
    }

    return Ok(ret);
}

fn parse_list(iter: &mut Iterator<Item = io::Result<u8>>) -> io::Result<Vec<BencodeValue>> {
    let mut iter = iter.peekable();
    iter.next(); // we don't need the "start of list" indicator
    let mut ret = Vec::new();

    loop {
        //TODO handle empty list "le"
        let val = parse_value(&mut iter)?;
        //println!("Adding {:?} to list", val);
        ret.push(val);

        let test = iter.peek().ok_or(Error::new(
            ErrorKind::InvalidData,
            "File ended while reading list",
        ))?;
        match *test {
            Ok(val) if val == 0x65 => break,
            Ok(_) => {}
            Err(ref err) => {
                println!("Error while reading list {}", err);
                return Err(Error::new(ErrorKind::Other, "Error while reading list"));
            }
        }
    }

    return Ok(ret);
}

fn parse_int(iter: &mut Iterator<Item = io::Result<u8>>) -> io::Result<i64> {
    let max_digits = 19;

    iter.next(); // we don't need the "start of integer" indicator

    let mut is_negative = false;
    let mut curr_byte;

    curr_byte = iter.next().ok_or(Error::new(
        ErrorKind::InvalidData,
        "File ended while reading integer.",
    ))??;

    if curr_byte == 0x2d {
        is_negative = true;
        curr_byte = iter.next().ok_or(Error::new(
            ErrorKind::InvalidData,
            "File ended while reading integer.",
        ))??;
    }

    let mut int_chars = Vec::with_capacity(19);

    loop {
        if int_chars.len() >= max_digits {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Integer is larger than 64 bytes.",
            ));
        } else if curr_byte == 0x65 {
            break;
        }
        if curr_byte >= 0x30 && curr_byte <= 0x39 {
            int_chars.push(curr_byte)
        } else {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Expected an integer (byte 0x30 - 0x39) got {:x}",
                    curr_byte
                ),
            ));
        }
        curr_byte = iter.next().ok_or(Error::new(
            ErrorKind::InvalidData,
            "File ended while reading integer.",
        ))??;
    }

    return Ok(vec_to_int(&int_chars, is_negative)?);
}

fn vec_to_int(vec: &Vec<u8>, is_negative: bool) -> io::Result<i64> {
    let mut ret: i64 = 0;

    // TODO checked math operations
    for val in vec {
        ret *= 10;
        ret += *val as i64 - 0x30;
    }

    if is_negative {
        ret *= -1;
    }

    Ok(ret)
}
