use std::process::exit;

use boxrec_tool::Config;

fn main() {
    let config = Config::new(std::env::args()).unwrap_or_else(|err| {
        eprintln!("Error parsing arguments: {}", err);
        exit(1);
    });

    if let Err(err) = boxrec_tool::run(config) {
        eprintln!("Error while running: {}", err);
        exit(2);
    }
}
