use std::{
    error::Error,
    process::exit,
    sync::Arc,
    thread::sleep,
};

use serenity::{
    async_trait,
    builder::CreateEmbed,
    http::Http,
    model::{
        channel::Message,
        gateway::{Activity, Ready},
        id::{ChannelId, GuildId},
        user::OnlineStatus
    },
    prelude::*,
    utils::Colour,
};

use boxrec_tool::{Notification, State};

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

    fn generate_embed(notif: Notification) -> CreateEmbed {
        let mut e = CreateEmbed::default();
        if notif.warning {
            e.colour(Colour::from_rgb(255, 0, 0));
        }
        e.title(format!("{} vs. {}", notif.winner_to_be, notif.loser_to_be));
        e.author(|a| {
            a.name("BoxRec Heuristic Tool");
            a.url("https://github.com/alpha-tango-kilo/boxrec-heuristic");
            a.icon_url("https://avatars3.githubusercontent.com/u/12728900");
            a
        });
        e.footer(|f| { f.text("Please gamble responsibly") });

        e.field(format!("Our odds ({} wins)", notif.winner_to_be.forename),
                notif.win_percent_ours,
                true
        );
        e.field(format!("Betfair odds ({} wins)", notif.winner_to_be.forename),
                format!("{} ({})", notif.betfair_odds.as_frac(), notif.betfair_odds.as_percent()),
                true
        );
        e
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let wrapper_task = async move |http: Arc<Http>, channels: Arc<Mutex<Vec<ChannelId>>>| -> String {
            let main_task = async move || -> Result<(), Box<dyn Error>> {
                let mut state = State::new()?;

                println!("{:?}", channels);

                state.read_cache()?;

                loop {
                    let notifs = state.task()?;
                    let channels = channels.lock().await;
                    for notif in notifs.into_iter()
                        .map(|n| { Bot::generate_embed(n) })
                    {
                        for channel in channels.iter() {
                            channel.send_message(&http, |m| {
                                m.embed(|e| {
                                    e.0 = notif.0.clone();
                                    e
                                })
                            }).await?;
                        }
                    }
                    state.write_cache()?;
                    sleep(state.get_recheck_delay());
                }
            };
            if let Err(why) = main_task().await {
                why.to_string()
            } else { String::from("If you see this panic") }
        };

        let exit_reason = tokio::spawn(wrapper_task(ctx.http.clone(), self.notify_channels.clone()));
        if let Ok(why) = exit_reason.await {
            eprintln!("Exiting: {}", why);
        }
        //ctx.shard.shutdown_clean();
        exit(3);
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
