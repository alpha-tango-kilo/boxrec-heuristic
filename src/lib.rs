#![allow(dead_code)]

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use BoutStatus::*;
use boxer::*;

use crate::betfair::{BetfairAPI, Bout};
use crate::boxrec::BoxRecAPI;
use crate::discord::Bot;

mod betfair;
mod boxer;
mod boxrec;
mod discord;

const CONFIG_PATH: &str = "./config.yml";

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub cache_path: Option<String>,     // directory path to store cache files
    pub username: Option<String>,       // username for BoxRec
    pub password: Option<String>,       // password for BoxRec
    request_delay: Option<u64>,         // minimum time between BoxRec requests
    notify_threshold: Option<f32>,      // positive difference in our odds required to get notified
    recheck_delay: Option<u16>,         // time in minutes between Betfair checks
}

impl Config {
    fn new(path: &str) -> Config {
        match fs::read_to_string(path) {
            Ok(contents) => match serde_yaml::from_str::<Config>(contents.as_str()) {
                Ok(config) => {
                    // Validate numbers
                    if let Some(percent) = &config.notify_threshold {
                        if percent < &0f32 || percent > &100f32 {
                            eprintln!("Config had bad notify_threshold, using default configuration (Read: {}%)", percent);
                            return Config::new_default();
                        }
                    }
                    config
                },
                Err(why) => {
                    eprintln!("Failed to parse config file, using default (Error: {})", why);
                    Config::new_default()
                },
            },
            Err(why) => {
                eprintln!("Failed to read config file, using default (Error: {})", why);
                Config::new_default()
            },
        }
    }

    fn new_default() -> Config {
        // Sensible defaults™
        Config {
            cache_path: Some(String::from("./.cache")), // Cache by default
            username: None,
            password: None,
            request_delay: Some(500u64),
            notify_threshold: Some(15f32),
            recheck_delay: Some(60u16),
        }
    }

    fn save(&self) -> Result<(), Box<dyn Error>> {
        let ser = serde_yaml::to_string(&self)?;
        match OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(CONFIG_PATH)?
            .write_all(ser.as_bytes())
        {
            Ok(_) => Ok(()),
            Err(why) => {
                eprintln!("Failed to save config file (Error: {})", why);
                eprintln!("Here's the config if you wanted it:\n{}", ser);
                Err(why.into())
            },
        }
    }

    pub fn get_request_delay(&self) -> Duration {
        let ms = match self.request_delay {
            Some(ms) => ms,
            None => Config::new_default().request_delay.unwrap(),
        };
        Duration::from_millis(ms)
    }

    pub fn get_notify_threshold(&self) -> f32 {
        match self.notify_threshold {
            Some(percent) => percent,
            None => Config::new_default().notify_threshold.unwrap(),
        }
    }

    fn get_recheck_delay(&self) -> Duration {
        let mins = match self.recheck_delay {
            Some(mins) => mins,
            None => Config::new_default().recheck_delay.unwrap(),
        };
        Duration::from_secs((mins * 60) as u64)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct BoutMetadata (
    Bout, // TODO: Maybe take a reference so memory usage isn't as ass?
    BoutStatus,
);

impl PartialEq for BoutMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
enum BoutStatus {
    MissingBoxers,
    MissingBoutPage,
    Checked,
    Announced,
}

impl BoutStatus {
    fn next(&mut self) {
        *self = match self {
            MissingBoxers => MissingBoutPage,
            MissingBoutPage => Checked,
            Checked => Announced,
            Announced => panic!("No next status (called on {})", self),
        };
    }
}

impl Display for BoutStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            BoutStatus::MissingBoxers => "Missing boxers",
            BoutStatus::MissingBoutPage => "Missing bout page",
            BoutStatus::Checked => "Odds compared between BoxRec & Betfair",
            BoutStatus::Announced => "User notified of odds differential",
        })
    }
}

fn compare_and_notify(matchup: &Matchup, bout: &Bout, threshold: &f32) -> BoutStatus {
    /*println!("Ours: {}%\tBetfair's:{}%\nOurs: {}%\tBetfair's:{}%",
             matchup.win_percent_one,
             bout.odds.one_wins.as_percent(),
             matchup.win_percent_two,
             bout.odds.two_wins.as_percent(),
    );*/
    if matchup.win_percent_one - bout.odds.one_wins.as_percent() > *threshold {
        pretty_print_notification(
            &matchup.fighter_one.get_name(),
            &matchup.win_percent_one,
            &matchup.fighter_two.get_name(),
            &bout.odds.one_wins.as_frac(),
            &matchup.warning,
        );
        BoutStatus::Announced
    } else if matchup.win_percent_two - bout.odds.two_wins.as_percent() > *threshold {
        pretty_print_notification(
            &matchup.fighter_two.get_name(),
            &matchup.win_percent_two,
            &matchup.fighter_one.get_name(),
            &bout.odds.two_wins.as_frac(),
            &matchup.warning,
        );
        BoutStatus::Announced
    } else {
        BoutStatus::Checked
    }
}

fn pretty_print_notification(winner_to_be: &str, win_percent: &f32, loser_to_be: &str, odds: &str, warning: &bool) {
    println!("---\n{}\
    We might be onto something chief!\n\
    BoxRec shows {} as having a {}% chance of winning against {}, and yet the betting odds are {}\n\
    ---",
             if *warning { "[WARNING: both boxer's have a BoxRec score below the safe threshold]\n" } else { "" },
             winner_to_be,
             win_percent,
             loser_to_be,
             odds
    );
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    // Get the bot going
    let token = match env::var("DISCORD_TOKEN") {
        Ok(t) => t,
        // TODO: allow user input
        Err(_) => return Err("Expected token in environment as DISCORD_TOKEN".into()),
    };
    let bot = Bot::new(&token).await?;

    let (res, _) = tokio::join!(bot.start(), bot.notify());
    res?;

    return Ok(());

    // Load config
    // TODO: make this changeable using a flag/arg
    let config = Config::new(CONFIG_PATH);

    // Connect to BoxRec
    let mut boxrec = BoxRecAPI::new(&config)?;
    boxrec.login()?;

    // Connect to Betfair
    let betfair = BetfairAPI::new()?;

    // Create Boxer HashMap (runtime cache/index of Boxers by name)
    let mut boxers: HashMap<String, Boxer> = HashMap::new();
    let mut bout_metadata: Vec<BoutMetadata> = Vec::new();

    // Load disk cache before running
    if let Some(cache_path) = &config.cache_path {
        read_cache(cache_path, &mut boxers, &mut bout_metadata).await?;
    }

    // Main running loop
    // Contents of this is run at program start, then every Config.recheck_delay minutes
    loop {
        // Scrape Betfair
        let bouts = betfair.get_listed_bouts()?;
        //println!("{:#?}", bouts);

        // Tag all bouts with metadata if they don't already have it
        bouts.into_iter()
            .for_each(|bout| {
                let bout = BoutMetadata(bout, MissingBoxers);
                if !bout_metadata.contains(&bout) { bout_metadata.push(bout); }
            });

        for BoutMetadata(bout, status) in bout_metadata.iter_mut() {
            // Step 1: Get boxers
            if status == &MissingBoxers {
                let mut have_one = true;
                let mut have_two = true;

                // If we don't have fighter one
                if !boxers.contains_key(&bout.fighter_one) {
                    // Look them up with BoxRec
                    if let Some(f1) = Boxer::new_by_name(&mut boxrec, &bout.fighter_one) {
                        // Insert them into the index if present
                        boxers.insert(bout.fighter_one.to_string(), f1);
                    } else {
                        have_one = false;
                    }
                }
                // If we don't have fighter two, same process as one
                if !boxers.contains_key(&bout.fighter_two) {
                    if let Some(f2) = Boxer::new_by_name(&mut boxrec, &bout.fighter_two) {
                        boxers.insert(bout.fighter_two.to_string(), f2);
                    } else {
                        have_two = false;
                    }
                }
                if have_one && have_two { status.next(); }
            }

            // Step 2: Get bout between boxers
            if status == &MissingBoutPage {
                let fighter_one = boxers.get(&bout.fighter_one).unwrap();
                let fighter_two = boxers.get(&bout.fighter_two).unwrap();

                let boxrec_odds = match fighter_one.get_bout_scores(&mut boxrec, &fighter_two) {
                    Ok(m) => m,
                    Err(why) => {
                        eprintln!("Failed to get bout between {} & {} (Error: {})",
                                  fighter_one.get_name(),
                                  fighter_two.get_name(),
                                  why);
                        continue;
                    },
                };
                status.next();

                compare_and_notify(&boxrec_odds, bout, &config.get_notify_threshold());
            }
        }

        // Save disk cache after running
        if let Some(cache_path) = &config.cache_path {
            write_cache(cache_path, &boxers, &bout_metadata).await?;
        }

        // Wait
        tokio::time::delay_for(config.get_recheck_delay()).await;
    }
    Ok(())
}

async fn read_cache(cache_path: &str, boxers: &mut HashMap<String, Boxer>, bout_metadata: &mut Vec<BoutMetadata>) -> Result<(), Box<dyn Error>> {
    // Check for and create cache folder
    match fs::metadata(cache_path) {
        // If Ok(), it exists
        // If it's a file, get scared, otherwise, we have a folder!
        Ok(md) => if md.is_file() {
            return Err("Cache path points to an existing file".into());
        },
        Err(why) => match why.kind() {
            // If the folder doesn't exist yet, try and make it
            ErrorKind::NotFound => fs::create_dir_all(cache_path)?,
            // If there's another error be spooked
            _ => return Err(why.into()),
        },
    };

    // Read pre-existing boxers cache if present and in a good format
    match fs::read_to_string(format!("{}/boxers.yml", cache_path)) {
        Ok(serialised) => serde_yaml::from_str::<Vec<Boxer>>(&serialised)?
            .into_iter()
            .for_each(|b| { boxers.insert(b.get_name(), b); }),
        Err(why) => match why.kind() {
            ErrorKind::NotFound => {},
            _ => return Err(why.into()),
        },
    };
    //println!("Read from disk cache into runtime index:\n{:#?}", boxers);

    // Read pre-existing bouts cache if present and in a good format
    match fs::read_to_string(format!("{}/bouts.yml", cache_path)) {
        Ok(serialised) => *bout_metadata = serde_yaml::from_str::<Vec<BoutMetadata>>(&serialised)?,
        Err(why) => match why.kind() {
            ErrorKind::NotFound => {},
            _ => return Err(why.into()),
        },
    };

    Ok(())
}

async fn write_cache(cache_path: &str, boxers: &HashMap<String, Boxer>, bout_metadata: &Vec<BoutMetadata>) -> Result<(), Box<dyn Error>> {
    let mut boxers_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(format!("{}/boxers.yml", cache_path))?;
    boxers_file.write(
        serde_yaml::to_string(
            &boxers.values()
                .collect::<Vec<_>>()
        )?.as_bytes()
    )?;

    let mut bouts_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(format!("{}/bouts.yml", cache_path))?;
    bouts_file.write(
        serde_yaml::to_string(
            &bout_metadata
        )?.as_bytes()
    )?;
    Ok(())
}
