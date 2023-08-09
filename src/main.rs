use core::fmt;
use std::io;

use hex::ToHex;

pub fn fmt_hex<T>(value: &T, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error>
where
    T: ToHex,
{
    formatter.write_str(&format!(r#""0x{}""#, value.encode_hex::<String>()))
}

pub fn fmt_hex_vec<T>(
    value: &Vec<T>,
    formatter: &mut std::fmt::Formatter,
) -> Result<(), std::fmt::Error>
where
    T: ToHex,
{
    let iter = value
        .iter()
        .map(|v| format!("0x{}", v.encode_hex::<String>()));
    formatter.debug_list().entries(iter).finish()
}

enum Parsed<'a> {
    Data(&'a [u8]),
    List(Vec<Parsed<'a>>),
}

impl fmt::Debug for Parsed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Parsed::Data(data) => fmt_hex(data, f),
            Parsed::List(items) => f.debug_list().entries(items).finish(),
        }
    }
}

impl fmt::Display for Parsed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(&self, f)
    }
}

fn var_len_be_to_usize(data: &[u8]) -> usize {
    if data.len() * 8 > usize::BITS as usize {
        panic!("Invalid length")
    }

    let mut out = 0usize;

    // Unfortunately we can't use usize::from_be_bytes or std::mem::transmute
    // (at least not easily), since the length is variable. (this is probably
    // inefficient, but that doesn't really matter for this project).
    for byte in data {
        out = (out << 8) + *byte as usize;
    }
    out
}

fn parse_rlp_list_internal(data: &[u8]) -> Vec<Parsed> {
    let mut items = vec![];
    let mut remaining = data;
    loop {
        let parsed;
        (parsed, remaining) = parse_rlp(remaining);
        items.push(parsed);
        if remaining.len() == 0 {
            break;
        }
    }
    items
}

fn parse_rlp(data: &[u8]) -> (Parsed, &[u8]) {
    match data[0] {
        0..=0x7f => (Parsed::Data(&data[0..=0]), &data[1..]),
        v @ 0x80..=0xb7 => {
            // 0-55 bytes
            let length = v as usize - 0x80;
            (Parsed::Data(&data[1..=length]), &data[1 + length..])
        }
        v @ 0xb8..=0xbf => {
            // More than 55 bytes
            let length_length = v as usize - 0xb7;
            let length = var_len_be_to_usize(&data[1..=length_length]);
            (
                Parsed::Data(&data[1 + length_length..=length_length + length]),
                &data[1 + length_length + length..],
            )
        }
        v @ 0xc0..=0xf7 => {
            // list with 0-55 bytes total
            let length = v as usize - 0xc0;
            let items = parse_rlp_list_internal(&data[1..=length]);
            (Parsed::List(items), &data[1 + length..])
        }
        v @ 0xf8..=0xff => {
            // List with more than 55 bytes total
            let length_length = v as usize - 0xf7;
            let length = var_len_be_to_usize(&data[1..=length_length]);
            let items = parse_rlp_list_internal(&data[1 + length_length..=length_length + length]);
            (Parsed::List(items), &data[1 + length + length_length..])
        }
    }
}

fn main() {
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();

    let line = line
        .strip_prefix("0x")
        .unwrap_or(&line)
        .strip_suffix("\n")
        .unwrap_or(&line);
    let data = hex::decode(line).unwrap();

    let (parsed, _) = parse_rlp(data.as_slice());
    println!("{parsed:#}");
}
