use std::env;
use std::process;

use gainlineup::cli;

fn main() {
    let args: Vec<String> = env::args().collect();

    let _ = cli::Config::run(&args).unwrap_or_else(|err| {
        println!();
        cli::print_error(&err.to_string()); //print at the top, but might be lost or hard to read
        println!();
        cli::print_help();
        println!();
        cli::print_error(&err.to_string()); // print error again, for human factors
        process::exit(1);
    });
}
