Thermite is an I/O generation tool written in Rust.

It can be used to stress-test storage, or to fill a device or file
with (optionally) non-repeating random data.

## Building

With debug symbols and no optimization:
`cargo build`

With optimizations and a reduced binary size:
`cargo build --release`

## Usage

```sh
./thermite -h
Options:
    -h, --help          print this help text
    -m, --mode          I/O mode, 'sequential' or 'random' or 'random100'
    -b, --blocksize     block size to write
    -p, --pagesize      page size over which to ensure uniqueness
    -f, --file /dev/sdX target file or block device
```

## License

Thermite is licensed under the terms of the GPL License (Version 3+).

See LICENSE for details.
