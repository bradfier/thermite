/*
 * Thermite - An I/O generation tool in Rust
 * Copyright (C) 2015 Richard Bradfield
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate getopts;
extern crate rand;
extern crate num;

use std::env;
use std::process;
use std::fs;
use rand::Rng;
use std::io::{Write,Seek,SeekFrom};
use getopts::Options;

mod lcg;

struct ThermiteOptions {
    blocksize: u64,
    pagesize: u64,
    target: String,
    mode: IOMode,
}

enum IOMode {
    Sequential,
    Random,
    Random100,
}

fn is_power2<T: num::PrimInt>(x: T) -> bool {
    let _0 = T::zero();
    let _1 = T::one();
    (x & x-_1) == _0
}

fn random_bytes(n: u32) -> Vec<u8> {
    (0..n).map(|_| rand::random::<u8>()).collect()
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
    opts.optopt("m", "mode", "I/O mode, 'sequential' or 'random' or 'random100'", "");
    opts.optopt("b", "blocksize", "block size to write", "");
    opts.optopt("p", "pagesize", "dedupe page-size (16384 for 3PAR)", "");
    opts.optopt("f", "file", "target file or block device", "/dev/sdX");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m },
        Err(f) => { panic!(f.to_string()) }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        process::exit(0);
    }

    let file_match = match matches.opt_str("f") {
        Some(x) => { x },
        None => {
            error_exit!(1, "File is a required parameter.");
        },
    };

    let mode_match = match matches.opt_str("m") {
        Some(x) => {
            match x.as_ref() {
                "sequential" => { IOMode::Sequential },
                "random" => { IOMode::Random },
                "random100" => { IOMode::Random100 },
                _ => {
                    error_exit!(1, "I/O Mode must be sequential or random or random100");
                }
            }
        },
        None => { IOMode::Random },
    };

    let blocksize_match = numeric_opt!(matches.opt_str("b"), u64, 512,
            "ERROR: Blocksize must be a positive power of 2.");
    let pagesize_match = numeric_opt!(matches.opt_str("p"), u64, 0,
            "ERROR: Pagesize must be a positive power of 2.");

    if (pagesize_match != 0) && (pagesize_match > blocksize_match) {
        error_exit!(1, "ERROR: Pagesize, if supplied, must be smaller than blocksize.");
    }
    if (pagesize_match != 0) && (!is_power2(pagesize_match)) {
        error_exit!(1, "ERROR: Pagesize must be a power of 2");
    }
    if !is_power2(blocksize_match) {
        error_exit!(1, "ERROR: Blocksize must be a power of 2");
    }

    ThermiteOptions {
        blocksize: blocksize_match,
        pagesize: pagesize_match,
        target: file_match,
        mode: mode_match,
    }
}

fn run_io(mut f: &fs::File, args: &ThermiteOptions) -> std::io::Result<()> {
    let end = f.seek(SeekFrom::End(0)).unwrap();
    let end_block = end / args.blocksize;

    let mut iterations = 0;
    let mut data: Vec<u8> = random_bytes(args.blocksize as u32);

    let seed = rand::thread_rng().gen_range::<u64>(0, end_block);
    let power2 = end_block.next_power_of_two();
    let mut generator = lcg::LCG::new(seed, power2);

    loop {

        let chosen_offset;

        match args.mode {
            IOMode::Random => {
                let random = rand::thread_rng().gen_range(0, end_block);
                chosen_offset = args.blocksize * random;
            },
            IOMode::Sequential => {
                chosen_offset = args.blocksize * iterations;
                if chosen_offset > (end_block * args.blocksize) {
                    break;
                }
            },
            IOMode::Random100 => {
                if iterations == end_block {
                    break;
                }
                chosen_offset = generator.next().unwrap() * args.blocksize;
            },
        };

        try!(f.seek(SeekFrom::Start(chosen_offset)));
        try!(f.write(&data[..]));

        xor_scramble(&mut data, args.pagesize, iterations);
        iterations += 1;
    }

    Ok(())
}

fn xor_scramble(data: &mut Vec<u8>, pagesize: u64, offset: u64) {
    let blocksize = data.len() as u64;

    if pagesize != 0 {
        let num_pages = blocksize / pagesize;
        let page_offsets: Vec<u64> =
                (0..num_pages).map(|x| x * pagesize).collect();

        for p_off in page_offsets {
            let this = offset & pagesize-1;
            let next = (offset + 1) & pagesize-1;
            let this_offset = this + p_off;
            let next_offset = next + p_off;

            data[this_offset as usize] ^= data[next_offset as usize];
        }
    } else {
        let this = offset & blocksize-1;
        let next = (offset + 1) & blocksize-1;

        data[this as usize] ^= data[next as usize];
    }
}

fn main() {

    // Argparse
    let args: Vec<String> = env::args().collect();
    let thermite_args = parse_opts(args);

    println!("Blocksize {}", thermite_args.blocksize);
    println!("Pagesize {}", thermite_args.pagesize);
    println!("Target {}", thermite_args.target);

    let mut options = fs::OpenOptions::new();
    options.read(true).write(true);

    let path = &thermite_args.target;

    let f = match options.open(path) {
        Ok(file) => { file },
        Err(_) => panic!("Could not open file {}", path),
    };

    // Drop the result from the IO as it's just an Ok unit 'Ok(())'
    let _ = run_io(&f, &thermite_args);
}
