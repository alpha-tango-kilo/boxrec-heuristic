use std::ops::Deref;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::{Activity, Ready}, id::ChannelId, user::OnlineStatus},
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

    async fn ready(&self, ctx: Context, ready: Ready) {
        let activity = Activity::playing("with BoxRec");
        let status = OnlineStatus::Online;
        ctx.set_presence(Some(activity), status);
        println!("{} is connected!", ready.user.name);
    }
}

pub struct Bot {
    discord: Client,
    notify_channel: Vec<ChannelId>,
}

impl Bot {
    pub async fn new(token: &str) -> Self {
        let mut discord = Client::new(token)
            .event_handler(Handler)
            .await
            .expect("Error building client");

        if let Err(err) = discord.start().await {
            println!("Bot error ({})", err);
        }

        Bot {
            discord,
            notify_channel: vec![],
        }
    }

    pub async fn notify(&self) {
        self.notify_channel.iter().for_each(|c| {

        });
    }
}

impl Deref for Bot {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.discord
    }
}
