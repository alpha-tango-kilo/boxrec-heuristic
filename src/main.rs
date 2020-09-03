#![feature(async_closure)]

use std::env;
use std::process::exit;

use crate::discord::Bot;

mod discord;

#[tokio::main]
async fn main() {
    // TODO: allow user input
    let token = env::var("DISCORD_TOKEN").unwrap_or_else(|_| {
        eprintln!("Expected token in environment as DISCORD_TOKEN");
        exit(1);
    });

    // Get the bot going
    Bot::new(&token).await.unwrap_or_else(|why| {
        eprintln!("{}", why);
        exit(2);
    });
}
