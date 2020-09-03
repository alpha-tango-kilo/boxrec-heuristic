use std::{
    error::Error,
    thread::sleep,
    time::Duration,
};

use serenity::{
    async_trait,
    http::Http,
    model::{channel::Message, gateway::{Activity, Ready}, id::ChannelId, user::OnlineStatus},
    prelude::*,
};

use boxrec_tool::State;

pub struct Bot {
    notify_channels: Mutex<Vec<ChannelId>>,
    task_running: Mutex<bool>,
    task_state: Mutex<State>,
}

impl Bot {
    pub async fn new(token: &str) -> Result<(), Box<dyn Error>> {
        let mut discord = Client::new(token)
            .event_handler(Bot {
                notify_channels: Mutex::new(vec![]),
                task_running: Mutex::new(false),
                task_state: Mutex::new(State::new()?)
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

        sleep(Duration::from_secs(20));

        let mut running = *self.task_running.lock().await;

        if !running {
            running = true;
            self.task_state.lock().await.read_cache();
            let http = ctx.http.clone();
            let task = async move |http| {
                println!("Test");
                let _foo = http;
                /*loop {
                    self.task_state.lock().await;
                }*/
            };
            let handle = tokio::spawn(task(http));
            handle.await;
        }
    }
}
