use std::process::exit;

fn main() {
    if let Err(err) = boxrec_tool::run() {
        eprintln!("Error while running: {}", err);
        exit(2);
    }
}
