use std::{
    fs::File,
    io::{BufRead, BufReader, SeekFrom},
    thread::yield_now,
    time::Instant,
};

use arc_reader::archive::Archive;
use clap::Parser;
use hash40::Hash40;

#[derive(Parser)]
pub enum Args {
    Load,
    Inspect { offset: String, how_much: String },
}

fn print_hex_values<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
    offset: u64,
    mut amount: usize,
) {
    use std::fmt::Write;
    let offset_aligned = offset & !0xF;
    amount += (offset & 0xF) as usize;

    reader.seek(SeekFrom::Start(offset_aligned)).unwrap();

    println!("             | 00 01 02 03 04 05 06 07   08 09 0A 0B 0C 0D 0E 0F");

    while amount > 0 {
        let pos = reader.stream_position().unwrap();

        let amount_this_line = amount.min(16);
        amount -= amount_this_line;

        let mut line = format!("0x{pos:0>10X} | ");

        let mut buffer = [0u8; 0x10];
        reader.read_exact(&mut buffer[..amount_this_line]).unwrap();

        for byte in &buffer[..8] {
            write!(&mut line, "{byte:02X} ").unwrap();
        }

        line.push_str("  ");

        for byte in &buffer[8..] {
            write!(&mut line, "{byte:02X} ").unwrap();
        }

        println!("{line}");
    }
}

fn main() {
    let args = Args::parse();

    match args {
        Args::Load => {
            Hash40::label_map()
                .lock()
                .unwrap()
                .add_labels_from_path("/Users/blujay/Downloads/Hashes_all")
                .unwrap();

            let mut file =
                BufReader::new(File::open("/Users/blujay/Downloads/13.0.1.arc").unwrap());

            let arc = Archive::read(&mut file).unwrap();

            let num_groups = arc.num_file_group();
            let (first, second) = arc.serialize_tables().unwrap();
            std::fs::write("./first.bin", first).unwrap();
            std::fs::write("./second.bin", second).unwrap();
        }
        Args::Inspect { offset, how_much } => {
            let offset = if offset.starts_with("0x") {
                u64::from_str_radix(offset.strip_prefix("0x").unwrap(), 16).unwrap()
            } else {
                offset.parse::<u64>().unwrap()
            };

            let how_much = if how_much.starts_with("0x") {
                usize::from_str_radix(how_much.strip_prefix("0x").unwrap(), 16).unwrap()
            } else {
                how_much.parse::<usize>().unwrap()
            };

            let mut file =
                BufReader::new(File::open("/Users/blujay/Downloads/13.0.1.arc").unwrap());

            print_hex_values(&mut file, offset, how_much);
        }
    }
}
