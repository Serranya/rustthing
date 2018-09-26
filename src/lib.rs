use std::collections::HashMap;

use std::io;
use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq)]
pub enum BencodeValue {
	Integer(i64),
	String(Vec<u8>),
	List(Vec<BencodeValue>),
	Dictionary(HashMap<Vec<u8>, BencodeValue>),
	EndOfFile,
}

pub fn parse_value(iter: &mut Iterator<Item = io::Result<u8>>) -> io::Result<BencodeValue> {
	let mut iter = iter.peekable();

	loop {
		let byte;
		match iter.peek() {
			Some(result) => match *result {
				Ok(val) => {
					byte = val;
				}
				Err(ref err) => {
					println!("Error while reading file {}", err);
					return Err(Error::new(ErrorKind::Other, "Error while reading file"));
				}
			},
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
				format!("Expected an integer (byte 0x30 - 0x39) got {:x}", curr_byte),
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

/// Parses an bencode list. The format is l<bencoded values>e for example
///
/// l4:spam4:eggse represents the list of two strings: [ "spam", "eggs" ]
/// le represents an empty list: []
fn parse_list(iter: &mut Iterator<Item = io::Result<u8>>) -> io::Result<Vec<BencodeValue>> {
	let mut iter = iter.peekable();
	iter.next(); // we don't need the "start of list" indicator

	if let Some(Ok(0x65)) = iter.peek() { // empty list
		return Ok(Vec::new()) // TODO use empty vec
	}

	let mut ret = Vec::new();

	loop {
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

/// Parses an bencode integer value. The format is
/// i<integer encoded in base ten ASCII>e we allowe some integers that are
/// forbidden by the specification. for example
///
/// i-0e // -0 is forbidden
/// i0123e // leading zeroes are forbidden (except i0e)
///
/// while the size of an integer is not stated in the spec we support up to 64 bits
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
				format!("Expected an integer (byte 0x30 - 0x39) got {:x}", curr_byte),
			));
		}
		curr_byte = iter.next().ok_or(Error::new(
			ErrorKind::InvalidData,
			"File ended while reading integer.",
		))??;
	}

	return Ok(vec_to_int(&int_chars, is_negative)?);
}

/// Parses the number given as ASCII in vec to an i64. Does not support
/// a sign. The sign must be passed via the is_negative parameter.
fn vec_to_int(vec: &Vec<u8>, is_negative: bool) -> io::Result<i64> {
	let mut ret: i64 = 0;

	for val in vec {
		if let Some(i) = ret.checked_mul(10) {
			ret = i;
		} else {
			return Err(Error::new(
				ErrorKind::InvalidData,
				"Integer field is longer i64",
			));
		}
		if let Some(i) = if is_negative {
			ret.checked_sub(*val as i64 - 0x30)
		} else {
			ret.checked_add(*val as i64 - 0x30)
		} {
			ret = i;
		} else {
			return Err(Error::new(
				ErrorKind::InvalidData,
				"Integer field is larger than i64",
			));
		}
	}

	Ok(ret)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_vec_to_int_positive() {
		assert_eq!(
			vec_to_int(&vec!['1' as u8, '2' as u8, '3' as u8], false).unwrap(),
			123
		);
	}

	#[test]
	fn test_vec_to_int_negative() {
		assert_eq!(
			vec_to_int(&vec!['1' as u8, '2' as u8, '3' as u8], true).unwrap(),
			-123
		);
	}

	#[test]
	fn test_vec_to_int_zero_prefix_positive() {
		assert_eq!(
			vec_to_int(&vec!['0' as u8, '2' as u8, '3' as u8], false).unwrap(),
			23
		);
	}

	#[test]
	fn test_vec_to_int_zero_prefix_negative() {
		assert_eq!(
			vec_to_int(&vec!['0' as u8, '2' as u8, '3' as u8], true).unwrap(),
			-23
		);
	}

	#[test]
	fn test_vec_to_int_zero_positive() {
		assert_eq!(
			vec_to_int(&vec!['0' as u8, '0' as u8, '0' as u8], false).unwrap(),
			0
		);
	}

	#[test]
	fn test_vec_to_int_zero_negative() {
		assert_eq!(
			vec_to_int(&vec!['0' as u8, '0' as u8, '0' as u8], true).unwrap(),
			-0
		);
	}

	#[test]
	fn test_vec_to_int_empty() {
		assert_eq!(vec_to_int(&vec![], true).unwrap(), 0);
	}

	#[test]
	fn test_vec_to_int_max_i64() {
		assert_eq!(
			vec_to_int(
				&vec![
					'9' as u8, '2' as u8, '2' as u8, '3' as u8, '3' as u8, '7' as u8, '2' as u8,
					'0' as u8, '3' as u8, '6' as u8, '8' as u8, '5' as u8, '4' as u8, '7' as u8,
					'7' as u8, '5' as u8, '8' as u8, '0' as u8, '7' as u8,
				],
				false,
			).unwrap(),
			9223372036854775807
		);
	}

	#[test]
	fn test_vec_to_int_max_i64_and_one_overflow() {
		assert!(
			vec_to_int(
				&vec![
					'9' as u8, '2' as u8, '2' as u8, '3' as u8, '3' as u8, '7' as u8, '2' as u8,
					'0' as u8, '3' as u8, '6' as u8, '8' as u8, '5' as u8, '4' as u8, '7' as u8,
					'7' as u8, '5' as u8, '8' as u8, '0' as u8, '8' as u8,
				],
				false,
			).is_err()
		);
	}

	#[test]
	fn test_vec_to_int_min_i64() {
		assert_eq!(
			vec_to_int(
				&vec![
					'9' as u8, '2' as u8, '2' as u8, '3' as u8, '3' as u8, '7' as u8, '2' as u8,
					'0' as u8, '3' as u8, '6' as u8, '8' as u8, '5' as u8, '4' as u8, '7' as u8,
					'7' as u8, '5' as u8, '8' as u8, '0' as u8, '8' as u8,
				],
				true,
			).unwrap(),
			-9223372036854775808
		);
	}

	#[test]
	fn test_vec_to_int_min_i64_and_minus_one_underflow() {
		assert!(
			vec_to_int(
				&vec![
					'9' as u8, '2' as u8, '2' as u8, '3' as u8, '3' as u8, '7' as u8, '2' as u8,
					'0' as u8, '3' as u8, '6' as u8, '8' as u8, '5' as u8, '4' as u8, '7' as u8,
					'7' as u8, '5' as u8, '8' as u8, '0' as u8, '9' as u8,
				],
				true,
			).is_err()
		);
	}

	#[test]
	fn test_parse_int_positive() {
		let mut val = String::from("i123e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert_eq!(parse_int(&mut val).unwrap(), 123);
	}

	#[test]
	fn test_parse_int_negative() {
		let mut val = String::from("i-123e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert_eq!(parse_int(&mut val).unwrap(), -123);
	}

	#[test]
	fn test_parse_int_zero_prefix_positive() {
		// leading zeroes are illegal according to the spec. Not sure if we should care
		// but since we don't error out make sure the result is at least "correct"
		let mut val = String::from("i0123e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert_eq!(parse_int(&mut val).unwrap(), 123);
	}

	#[test]
	fn test_parse_int_zero_prefix_negative() {
		// leading zeroes are illegal according to the spec. Not sure if we should care
		// but since we don't error out make sure the result is at least "correct"
		let mut val = String::from("i-0123e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert_eq!(parse_int(&mut val).unwrap(), -123);
	}

	#[test]
	fn test_parse_int_zero_positive() {
		let mut val = String::from("i0e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert_eq!(parse_int(&mut val).unwrap(), 0);
	}

	#[test]
	fn test_parse_int_zero_negative() {
		// negative zero is illegal according to the spec. Not sure if we should care
		// but since we don't error out make sure the result is at least "correct"
		let mut val = String::from("i-0e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert_eq!(parse_int(&mut val).unwrap(), -0);
		let mut val = String::from("i-0e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert_eq!(parse_int(&mut val).unwrap(), 0);
	}

	#[test]
	fn test_parse_int_empty() {
		// not sure if this is legal. The spec says that integers have no size limitation whatever
		// that means regarding to empty integers. Treating them as 0 seems sensible.
		let mut val = String::from("ie").into_bytes().into_iter().map(|byte| {
			return Ok(byte);
		});
		assert_eq!(parse_int(&mut val).unwrap(), 0);
	}

	#[test]
	fn test_parse_int_double_start() {
		let mut val = String::from("ii123e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert!(parse_int(&mut val).is_err());
	}

	#[test]
	fn test_parse_int_invalid_digit() {
		let mut val = String::from("i12x3e")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert!(parse_int(&mut val).is_err());
	}

	#[test]
	fn test_parse_int_missing_end() {
		let mut val = String::from("i123")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert!(parse_int(&mut val).is_err());
	}

	#[test]
	fn test_parse_list_empty() {
		let mut val = String::from("le")
			.into_bytes()
			.into_iter()
			.map(|byte| Ok(byte));
		assert!(parse_list(&mut val).is_ok());
	}
}
