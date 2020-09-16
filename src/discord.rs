use std::{
    error::Error,
    process::exit,
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use serenity::{
    async_trait,
    http::Http,
    model::{
        channel::Message,
        gateway::{Activity, Ready},
        id::{ChannelId, GuildId},
        user::OnlineStatus
    },
    prelude::*,
};

use boxrec_tool::State;

pub struct Bot {
    notify_channels: Arc<Mutex<Vec<ChannelId>>>,
}

impl Bot {
    pub async fn new(token: &str) -> Result<(), Box<dyn Error>> {
        let mut discord = Client::new(token)
            .event_handler(Bot {
                notify_channels: Arc::new(Mutex::new(vec![])),
            })
            .await?;

        discord.start().await?;

        Ok(())
    }

    async fn notify(&self, http: Http) {
        println!("Waiting!");
        tokio::time::delay_for(std::time::Duration::from_secs(30)).await;
        println!("Notifying!");
        let cs = self.notify_channels.lock().await;
        println!("{:?}", *cs);
        for c in cs.iter() {
            if let Err(why) = c.say(&http, "Foo").await {
                eprintln!("Failed to send notification to {} (Error: {}", c, why);
            } else {
                println!("Notification sent to {}", c);
            }
        }
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let fut = async move |channels| {
            if let Err(why) = || -> Result<(), Box<dyn Error>> {
                let mut state = State::new()?;
                let _http = ctx.http.clone();

                println!("{:?}", channels);

                state.read_cache()?;

                loop {
                    state.task()?;
                    state.write_cache()?;
                    sleep(state.get_recheck_delay());
                }
            }() {
                eprintln!("Error in task ({})", why);
                exit(3);
            }
        };

        tokio::spawn(fut(self.notify_channels.clone()));
    }

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
