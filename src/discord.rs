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

use boxrec_tool::boxer::Matchup;
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

    fn generate_embed(matchup: Matchup) -> CreateEmbed {
        let mut e = CreateEmbed::default();
        if matchup.warning {
            e.colour(Colour::from_rgb(255, 0, 0));
        }
        e.title(format!("{} vs. {}", matchup.fighter_one, matchup.fighter_two));
        e.author(|a| {
            a.name("BoxRec Heuristic Tool");
            a.url("https://github.com/alpha-tango-kilo/boxrec-heuristic");
            a.icon_url("https://avatars3.githubusercontent.com/u/12728900");
            a
        });
        e.footer(|f| { f.text("Please gamble responsibly") });

        e.field(format!("Our odds ({} wins)", matchup.fighter_one.forename),
            matchup.win_percent_one,
            true
        );
        e.field(format!("Betfair odds ({} wins)", matchup.fighter_one.forename),
            matchup.win_percent_one,
            true
        );
        e
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let main_task = async move |channels| {
            if let Err(why) = || -> Result<(), Box<dyn Error>> {
                let mut state = State::new()?;
                let _http = ctx.http.clone();

                println!("{:?}", channels);

                state.read_cache()?;

                loop {
                    let _notifs = state.task()?;
                    state.write_cache()?;
                    sleep(state.get_recheck_delay());
                }
            }() {
                eprintln!("Error in task ({})", why);
                exit(3);
            }
        };

        tokio::spawn(main_task(self.notify_channels.clone()));
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
