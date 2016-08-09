// Thermite - An I/O generation tool in Rust
// Copyright (C) 2015 Richard Bradfield
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//

extern crate getopts;
extern crate rand;
extern crate num;
#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::env;
use std::process;
use std::fs;
use rand::Rng;
use std::io::{Write, Seek, SeekFrom};
use std::time::Instant;
use std::thread;
use std::sync::{Arc, Mutex};
use getopts::Options;
use std::ops::IndexMut;

mod lcg;
mod watchdog;
mod logger;

struct ThermiteOptions {
    blocksize: u64,
    pagesize: u64,
    target: Vec<String>,
    mode: IOMode,
    startblock: u64,
    endblock: u64,
    data: DataType,
    interval: u64,
}

pub struct FileTarget {
    target: String,
    fd: fs::File,
}

#[derive(PartialEq)]
enum IOMode {
    Sequential,
    SequentialReverse,
    Random,
    Random100,
}

#[derive(PartialEq)]
enum DataType {
    Random,
    Zero,
}

fn random_bytes(n: usize) -> Vec<u8> {
    rand::thread_rng().gen_iter().filter(|&b| b != 0).take(n).collect()
}

#[inline(always)]
fn zero(n: usize) -> Vec<u8> {
    vec![0; n]
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

macro_rules! error_exit {
    ($errno:expr, $reason:expr) => {
        println!($reason);
        process::exit($errno);
    };
}

macro_rules! numeric_opt {
    ($matched:expr, $parse_type:ty, $default:expr, $error:expr) => {
        match $matched {
            Some(x) => {
                match x.parse::<$parse_type>() {
                    Ok(x) => {
                        if x == 0 {
                            error_exit!(1, $error);
                        } else { x }
                    },
                    Err(_) => {
                        error_exit!(1, $error);
                    },
                }
            },
            None => { $default },
        };
    };
}

fn parse_opts(args: Vec<String>) -> ThermiteOptions {
    // TODO Parameterize the defaults for the arguments
    let program = args[0].clone();

    let mut opts = Options::new();

    opts.optflag("h", "help", "print this help text");
    opts.optopt("m",
                "mode",
                "I/O mode, 'sequential' or 'sequentialreverse'  or 'random' or 'random100'",
                "");
    opts.optopt("d", "data", "datatype, 'random' or 'zero'", "");
    opts.optopt("s",
                "startblock",
                "the starting block given the specified blocksize",
                "");
    opts.optopt("e",
                "endblock",
                "the ending block given the specified blocksize",
                "");
    opts.optopt("b", "blocksize", "block size to write", "");
    opts.optopt("p", "pagesize", "dedupe page-size (16384 for 3PAR)", "");
    opts.optopt("i",
                "interval",
                "number of blocks to skip between write ops",
                "");
    opts.optmulti("f", "file", "target file or block device", "/dev/sdX");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        process::exit(0);
    }

    let files_match = match matches.opt_strs("f").len() {
        0 => {
            error_exit!(1, "File is a required parameter.");
        }
        _ => matches.opt_strs("f"),
    };

    let mode_match = match matches.opt_str("m") {
        Some(x) => {
            match x.as_ref() {
                "sequential" => IOMode::Sequential,
                "sequentialreverse" => IOMode::SequentialReverse,
                "random" => IOMode::Random,
                "random100" => IOMode::Random100,
                _ => {
                    error_exit!(1, "I/O Mode must be sequential or random or random100");
                }
            }
        }
        None => IOMode::Random,
    };

    let data_match = match matches.opt_str("d") {
        Some(y) => {
            match y.as_ref() {
                "random" => DataType::Random,
                "zero" => DataType::Zero,
                _ => {
                    error_exit!(1, "Data type must be random or zero");
                }
            }
        }
        None => DataType::Random,
    };

    let blocksize_match = numeric_opt!(matches.opt_str("b"),
                                       u64,
                                       512,
                                       "ERROR: Blocksize must be a positive power of 2.");
    let pagesize_match = numeric_opt!(matches.opt_str("p"),
                                      u64,
                                      0,
                                      "ERROR: Pagesize must be a positive power of 2.");
    let startblock_match = numeric_opt!(matches.opt_str("s"),
                                        u64,
                                        0,
                                        "ERROR: startblock must be a number.");
    let endblock_match = numeric_opt!(matches.opt_str("e"),
                                      u64,
                                      0,
                                      "ERROR: endblock must be a number.");
    let interval_match = numeric_opt!(matches.opt_str("i"),
                                      u64,
                                      0,
                                      "ERROR: block skip interval must be number.");

    if (pagesize_match != 0) && (pagesize_match > blocksize_match) {
        error_exit!(1,
                    "ERROR: Pagesize, if supplied, must be smaller than blocksize.");
    }
    if (pagesize_match != 0) && (!pagesize_match.is_power_of_two()) {
        error_exit!(1, "ERROR: Pagesize must be a power of 2");
    }
    if !blocksize_match.is_power_of_two() {
        error_exit!(1, "ERROR: Blocksize must be a power of 2");
    }
    if (endblock_match != 0) && (endblock_match < startblock_match) {
        error_exit!(1, "ERROR: Endblock must be higher than startblock");
    }


    ThermiteOptions {
        blocksize: blocksize_match,
        pagesize: pagesize_match,
        target: files_match,
        mode: mode_match,
        startblock: startblock_match,
        endblock: endblock_match,
        data: data_match,
        interval: interval_match,
    }
}

fn run_io(args: &ThermiteOptions) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true);

    let mut file_targets: Vec<FileTarget> = args.target
        .as_slice()
        .into_iter()
        .map(|f| match options.open(f) {
            Ok(file) => {
                FileTarget {
                    fd: file,
                    target: f.to_string(),
                }
            }
            Err(_) => panic!("Could not open file {}", f),
        })
        .collect();

    // Check that all the supplied file descriptors are trivially the same length
    let length = file_targets.index_mut(0).fd.seek(SeekFrom::End(0)).unwrap();
    for file_target in &mut file_targets {
        if file_target.fd.seek(SeekFrom::End(0)).unwrap() != length {
            error_exit!(1, "Supplied target files are different sizes!");
        }
    }


    let end = file_targets.index_mut(0).fd.seek(SeekFrom::End(0)).unwrap();
    let mut end_block = end / args.blocksize;
    if args.endblock != 0 {
        end_block = args.endblock;
    }
    let mut start_block = 0;
    if args.startblock != 0 {
        start_block = args.startblock;
    }

    let blockskip = args.interval;

    info!("File length in blocks {}", end / args.blocksize);
    info!("Start_Block {}", start_block);
    info!("End_Block {}", end_block);
    info!("Block Skip Interval: {}", blockskip);

    let mut iterations = 0;
    let mut data: Vec<u8>;
    match args.data {
        DataType::Random => {
            data = random_bytes(args.blocksize as usize);
        }
        DataType::Zero => {
            data = zero(args.blocksize as usize);
        }
    };

    let seed = rand::thread_rng().gen_range::<u64>(start_block, end_block);
    let power2 = (end_block - start_block).next_power_of_two();
    let mut generator = lcg::LCG::new(seed, power2);

    // Watchdog shared memory
    let last_io_times = Arc::new(Mutex::new(HashMap::new()));
    for ft in &file_targets {
        let mut map = last_io_times.lock().unwrap();
        map.insert(ft.target.clone(), Instant::now());
    }
    let shared = last_io_times.clone();
    thread::spawn(move || {
        watchdog::watch(shared, 2u64, 3u64);
    });

    loop {

        let chosen_offset;

        match args.mode {
            IOMode::Random => {
                let random = rand::thread_rng().gen_range(start_block, end_block);
                chosen_offset = args.blocksize * random;
            }
            IOMode::Sequential => {
                chosen_offset = (args.blocksize * iterations) + (start_block * args.blocksize);
                if chosen_offset > (end_block * args.blocksize) {
                    break;
                }
            }
            IOMode::SequentialReverse => {
                chosen_offset = (args.blocksize * (end_block - 1)) - (args.blocksize * iterations);
                if chosen_offset <= start_block * args.blocksize {
                    break;
                }
            }
            IOMode::Random100 => {
                if iterations == end_block {
                    break;
                }
                let mut random = generator.next().unwrap();
                while random >= end_block {
                    random = generator.next().unwrap();
                }
                chosen_offset = (random * args.blocksize) + (start_block * args.blocksize);
            }
        };

        for mut ft in &mut file_targets {
            try!(ft.fd.seek(SeekFrom::Start(chosen_offset)));
            try!(ft.fd.write(&data[..]));
            let mut last_io_guard = last_io_times.lock().unwrap();
            if let Some(x) = last_io_guard.get_mut(&ft.target) {
                *x = Instant::now();
            }
        }

        xor_scramble(&mut data, args.pagesize, iterations);
        iterations += 1 + blockskip;
    }

    Ok(())
}

fn xor_scramble(data: &mut Vec<u8>, pagesize: u64, offset: u64) {
    let blocksize = data.len() as u64;

    if pagesize != 0 {
        let num_pages = blocksize / pagesize;
        let page_offsets: Vec<u64> = (0..num_pages).map(|x| x * pagesize).collect();

        for p_off in page_offsets {
            let this = offset & (pagesize - 1);
            let next = (offset + 1) & (pagesize - 1);
            let this_offset = this + p_off;
            let next_offset = next + p_off;

            data[this_offset as usize] ^= data[next_offset as usize];
        }
    } else {
        let this = offset & (blocksize - 1);
        let next = (offset + 1) & (blocksize - 1);

        data[this as usize] ^= data[next as usize];
    }
}

fn main() {
    // Logging setup
    logger::init().unwrap();

    // Argparse
    let args: Vec<String> = env::args().collect();
    let thermite_args = parse_opts(args);

    info!("Blocksize: {}", thermite_args.blocksize);
    info!("Pagesize: {}", thermite_args.pagesize);
    for t in &thermite_args.target {
        info!("Target found: {} ", t);
    }

    // Drop the result from the IO as it's just an Ok unit 'Ok(())'
    let _ = run_io(&thermite_args);
}
