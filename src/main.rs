extern crate getopts;
extern crate rand;

use std::thread;
use std::env;
use std::process;
use getopts::Options;

struct ThermiteOptions {
    threads: u8,
    blocksize: usize,
    pagesize: usize,
    target: String,
}

fn is_power2(x: u32) -> bool {
    (x & x-1) == 0
}

fn random_bytes(n: u32) -> Vec<u8> {
    (0..n).map(|_| rand::random::<u8>()).collect()
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

macro_rules! opt_with_default {
    ($matched:expr, $parse_type:ty, $default:expr, $error:expr) => {
        match $matched {
            Some(x) => {
                match x.parse::<$parse_type>() {
                    Ok(x) => { x },
                    Err(_) => {
                        println!($error);
                        process::exit(1)
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
    opts.optopt("t", "threads", "number of I/O threads","");
    opts.optopt("b", "blocksize", "block size to write","");
    opts.optopt("p", "pagesize", "dedupe page-size (16384 for 3PAR)","");
    opts.optopt("f", "file", "target file or block device","/dev/sdX");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m },
        Err(f) => { panic!(f.to_string()) }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        process::exit(1);
    }

    let file_match = match matches.opt_str("f") {
        Some(x) => { x },
        None => {
            println!("File is a required parameter.");
            process::exit(1)
        },
    };

    let thread_match = opt_with_default!(matches.opt_str("t"), u8, 1,
            "ERROR: Threads must be a numeric value between 1 and 255.");
    let blocksize_match = opt_with_default!(matches.opt_str("b"), usize, 512,
            "ERROR: Blocksize must be a positive power of 2.");
    let pagesize_match = opt_with_default!(matches.opt_str("p"), usize, 0,
            "ERROR: Pagesize must be a positive power of 2.");

    if (pagesize_match != 0) && (pagesize_match > blocksize_match) {
        println!("ERROR: Pagesize, if supplied, must be smaller than blocksize.");
        process::exit(1);
    }

    ThermiteOptions {
        threads: thread_match,
        blocksize: blocksize_match,
        pagesize: pagesize_match,
        target: file_match,
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
