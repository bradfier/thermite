extern crate getopts;
extern crate rand;

use std::thread;
use std::env;
use std::process;
use getopts::Options;

struct ThermiteOptions<'a> {
    threads: u8,
    blocksize: usize,
    pagesize: usize,
    target: &'a str,
}

fn is_power2(x: u32) -> bool {
    (x & x-1) == 0
}

fn random_bytes(n: u32) -> Vec<u8> {
    (0..n).map(|_| rand::random::<u8>()).collect()
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] TARGET", program);
    print!("{}", opts.usage(&brief));
}

fn parse_opts<'a>(args: Vec<String>) -> ThermiteOptions<'a> {
    // TODO Parameterize the defaults for the arguments
    let program = args[0].clone();

    let mut opts = Options::new();

    opts.optopt("t", "threads", "number of I/O threads","");
    opts.optopt("b", "blocksize", "block size to write","");
    opts.optopt("p", "pagesize", "dedupe page-size (16384 for 3PAR)","");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m },
        Err(f) => { panic!(f.to_string()) }
    };

    let thread_arg = matches.opt_str("t");
    let thread_match = match thread_arg {
        Some(x) => {
            match x.parse::<u8>() {
                Ok(x) => { x },
                Err(_) => {
                    println!("ERROR: Threads argument must be an \
                             integer between 1 and 255");
                    process::exit(1)
                },
            }
        },
        None => { 1 },
    };

    let blocksize_match = match matches.opt_str("b") {
        Some(x) => {
            match x.parse::<usize>() {
                Ok(x) => { x },
                Err(_) => {
                    println!("ERROR: Blocksize must be a numeric value.");
                    process::exit(1)
                },
            }
        },
        None => { 512 },
    };

    let pagesize_match = match matches.opt_str("p") {
        Some(x) => {
            match x.parse::<usize>() {
                Ok(x) => {
                    match x <= blocksize_match {
                        true => { x },
                        false => {
                            println!("Pagesize must exceed blocksize.");
                            process::exit(1)
                        },
                    }
                },
                Err(_) => {
                    println!("ERROR: Unknwon");
                    process::exit(1)
                },
            }
        },
        None => { 0 },
    };

    ThermiteOptions {
        threads: thread_match,
        blocksize: blocksize_match,
        pagesize: pagesize_match,
        target: "/home/bradfier/file.bin",
    }
}

fn main() {

    // Argparse
    let args: Vec<String> = env::args().collect();

    let thermite_args = parse_opts(args);

    println!("Threads {}", thermite_args.threads);
    println!("Blocksize {}", thermite_args.blocksize);
    println!("Pagesize {}", thermite_args.pagesize);
    println!("Target {}", thermite_args.target);

}
