use std::sync::Arc;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::{Activity, Ready}, id::ChannelId, user::OnlineStatus},
    prelude::*,
};

struct Handler {
    notify_channels: Arc<Mutex<Vec<ChannelId>>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "`ping`" {
            if let Err(err) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message ({})", err);
            }
        } else if msg.content == "`notify`" {
            self.notify_channels.lock().await.push(msg.channel_id);
            println!("Will now send messages to {}", msg.channel_id);
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        let activity = Activity::playing("with BoxRec");
        let status = OnlineStatus::Online;
        ctx.set_presence(Some(activity), status).await;
        println!("{} is connected!", ready.user.name);
    }
}

pub struct Bot {
    discord: Client,
    notify_channels: Arc<Mutex<Vec<ChannelId>>>,
}

impl Bot {
    pub async fn new(token: &str) -> Self {
        let notify_channels = Arc::new(Mutex::new(vec![]));
        let mut discord = Client::new(token)
            .event_handler(Handler { notify_channels: notify_channels.clone() })
            .await
            .expect("Error building client");

        if let Err(err) = discord.start().await {
            println!("Bot error ({})", err);
        }

        Bot {
            discord,
            notify_channels,
        }
    }

    pub async fn notify(&self) {
        println!("Notifying!");
        for c in self.notify_channels.lock().await.iter() {
            if let Err(why) = c.say(&self.discord.cache_and_http.http, "Foo").await {
                eprintln!("Failed to send notification to {} (Error: {}", c, why);
            } else {
                println!("Notification sent to {}", c);
            }
        }
    }
}
