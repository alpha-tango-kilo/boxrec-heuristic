#![allow(dead_code, unused_variables)]

use std::error::Error;
use std::fs::{self, File};
use std::io::Write;

use serde::{Deserialize, Serialize};

use boxer::*;

use crate::betfair::{BetfairAPI, Bout};
use crate::boxrec::BoxRecAPI;

mod betfair;
mod boxer;
mod boxrec;

const CONFIG_PATH: &str = "./config.yaml";
const NOTIFY_THRESHOLD: f32 = 25f32; // TODO: Add to config file

pub struct Args {
    pub name_one: String,
    pub name_two: String,
}

impl Args {
    pub fn new(mut args: std::env::Args) -> Result<Args, Box<dyn Error>> {
        args.next();

        Ok(Args {
            name_one: args.next()
                .ok_or("Missing boxers' names")?,
            name_two: args.next()
                .ok_or("Missing second boxer's name")?,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub data_path: String,
    pub cache_path: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Config {
    fn new(path: &str) -> Config {
        match fs::read_to_string(path) {
            Ok(contents) => match serde_yaml::from_str(contents.as_str()) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("Failed to parse config file, using default (Error: {})", err);
                    Config::new_default()
                }
            },
            Err(err) => {
                eprintln!("Failed to read config file, using default (Error: {})", err);
                Config::new_default()
            },
        }
    }

    fn new_default() -> Config {
        Config {
            data_path: String::from("./data"),
            cache_path: Some("./cache.csv".to_string()), // Cache by default
            username: None,
            password: None,
        }
    }

    fn save(self) -> Result<(), Box<dyn Error>> {
        let se = serde_yaml::to_string(&self).unwrap();
        match File::create(CONFIG_PATH)?.write_all(se.as_bytes()) {
            Ok(_) => Ok(()),
            Err(err) => {
                eprintln!("Failed to save config file (Error: {})", err);
                Err(err.into())
            },
        }
    }
}

fn compare_and_notify(matchup: &Matchup, bout: &Bout, threshold: &f32) {
    if matchup.win_percent_one - bout.odds.one_wins.as_percent() > *threshold {
        pretty_print_notification(
            &matchup.fighter_one.get_name(),
            &matchup.win_percent_one,
            &matchup.fighter_two.get_name(),
            &bout.odds.one_wins.as_frac(),
            &matchup.warning,
        );
    } else if matchup.win_percent_two - bout.odds.two_wins.as_percent() > *threshold {
        pretty_print_notification(
            &matchup.fighter_two.get_name(),
            &matchup.win_percent_two,
            &matchup.fighter_one.get_name(),
            &bout.odds.two_wins.as_frac(),
            &matchup.warning,
        );
    }
}

fn pretty_print_notification(winner_to_be: &str, win_percent: &f32, loser_to_be: &str, odds: &str, warning: &bool) {
    println!("---\
        {}We might be onto something chief!\
        BoxRec shows {} as having a {}% chance of winning against {}, and yet the betting odds are {}\
        ---",
             if *warning { "[WARNING: both boxer's have a BoxRec score below the safe threshold]\n" } else { "" },
             winner_to_be,
             win_percent,
             loser_to_be,
             odds
    );
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    // TODO: make this changeable using a flag
    let config = Config::new(CONFIG_PATH);

    //let client = BoxRecAPI::new()?;
    //client.login(&config)?;

    let betfair = BetfairAPI::new()?;
    let bouts = betfair.get_listed_bouts()?;
    println!("{:#?}", bouts);

    /*match Boxer::new_by_id(&client, 314868) {
        Some(b) => println!("It worked! {:?}", b),
        None => println!("It didn't work :("),
    };*/

    // API stuffs
    //client.get_page_by_id(&client, 629465)?;
    //client.boxer_search(&client, "Floyd", "Mark", false)?;
    //let bout_odds = client.get_bout_odds(&626585, "ted cheeseman")?;
    //println!("{:?}", bout_odds);

    // If caching is enabled, do things here
    if let Some(cache_path) = &config.cache_path {

    }

    //config.save()?;
    Ok(())
}
