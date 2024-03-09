mod lasers;
mod huffman_code;
use std::fs::read_to_string;

fn main() {
    lasers::do_laser(read_to_string("src/lasers.rs").unwrap());
}
