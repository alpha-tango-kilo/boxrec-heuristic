use std::env;
use std::process::exit;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "`ping`" {
            if let Err(err) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message ({})", err);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Get the bot going
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected token in environment as DISCORD_TOKEN");


    let mut client = Client::new(&token)
        .event_handler(Handler)
        .await
        .expect("Error building client");

    if let Err(err) = client.start().await {
        println!("Client error ({})", err);
    }

    /*if let Err(err) = boxrec_tool::run() {
        eprintln!("Error while running: {}", err);
        exit(2);
    }*/
}
