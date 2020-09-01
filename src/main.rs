use std::process::exit;

#[tokio::main]
async fn main() {
    if let Err(why) = boxrec_tool::run().await {
        eprintln!("Error while running: {}", why);
        exit(2);
    }
}
