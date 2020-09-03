use std::error::Error;
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
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message ({})", why);
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
    pub async fn new(token: &str) -> Result<Self, Box<dyn Error>> {
        let notify_channels = Arc::new(Mutex::new(vec![]));
        let mut discord = Client::new(token)
            .event_handler(Handler { notify_channels: notify_channels.clone() })
            .await?;

        discord.start().await?;

        Ok(Bot {
            discord,
            notify_channels,
        })
    }

    pub async fn notify(&self) {
        println!("Waiting!");
        tokio::time::delay_for(std::time::Duration::from_secs(30)).await;
        println!("Notifying!");
        let cs = self.notify_channels.lock().await;
        println!("{:?}", *cs);
        for _c in cs.iter() {
            /*
            if let Err(why) = c.say(, "Foo").await {
                eprintln!("Failed to send notification to {} (Error: {}", c, why);
            } else {
                println!("Notification sent to {}", c);
            }
            */
        }
    }
}
