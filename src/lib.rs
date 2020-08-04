#![allow(dead_code, unused_variables)]

use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
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
            cache_path: Some("./cache.yml".to_string()), // Cache by default
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

pub fn run() -> Result<(), Box<dyn Error>> {
    // TODO: make this changeable using a flag
    let config = Config::new(CONFIG_PATH);

    let client = BoxRecAPI::new()?;
    client.login(&config)?;

    let betfair = BetfairAPI::new()?;
    let bouts = betfair.get_listed_bouts()?;
    //println!("{:#?}", bouts);

    let mut boxers: HashMap<String, Boxer> = HashMap::new();

    for bout in bouts.iter() {
        /*
        I probably should document this abomination
        Basically, Rust won't let you insert into a map while a reference is held
        The premise of the below therefore is to test the waters first - check if the two boxers are contained, then make sure to create and add any that aren't in the map before getting references for them
         */
        // To start, we're matching against a bool tuple of whether each boxer is already known
        let (fighter_one, fighter_two) = match (
            boxers.contains_key(&bout.fighter_one),
            boxers.contains_key(&bout.fighter_two))
        {
            // If both are known, no insertions need to happen, we chill
            (true, true) => (boxers.get(&bout.fighter_one).unwrap(),
                             boxers.get(&bout.fighter_two).unwrap()),
            // If one is unknown
            (true, false) => {
                // Try and create the unknown one
                match Boxer::new_by_name(&client, &bout.fighter_two) {
                    // If it works
                    Some(f2) => {
                        // Insert it into the map
                        boxers.insert(bout.fighter_two.to_string(), f2);
                        // Then get both references
                        (boxers.get(&bout.fighter_one).unwrap(),
                         boxers.get(&bout.fighter_two).unwrap())
                    }
                    // If it doesn't work, skip this bout
                    None => continue,
                }
            }
            // Same case as above, but other boxer is unknown
            (false, true) => {
                match Boxer::new_by_name(&client, &bout.fighter_one) {
                    Some(f1) => {
                        boxers.insert(bout.fighter_one.to_string(), f1);
                        (boxers.get(&bout.fighter_one).unwrap(),
                         boxers.get(&bout.fighter_two).unwrap())
                    }
                    None => continue,
                }
            }
            // If neither boxer is known
            (false, false) => {
                // Try and make the first
                match Boxer::new_by_name(&client, &bout.fighter_one) {
                    // If it worked, insert
                    Some(f1) => boxers.insert(bout.fighter_one.to_string(), f1),
                    // Skip this bout otherwise
                    None => continue,
                };
                match Boxer::new_by_name(&client, &bout.fighter_two) {
                    // If it worked, insert
                    Some(f2) => boxers.insert(bout.fighter_two.to_string(), f2),
                    // Skip this bout if it didn't (at least we still have one more dude documented)
                    None => continue,
                };
                // Get both references
                (boxers.get(&bout.fighter_one).unwrap(),
                 boxers.get(&bout.fighter_two).unwrap())
            }
        };

        let boxrec_odds = match fighter_one.get_bout_scores(&client, &fighter_two) {
            Ok(m) => m,
            Err(err) => {
                eprintln!("Failed to get matchup between {} & {}",
                          fighter_one.get_name(),
                          fighter_two.get_name());
                continue;
            },
        };
        compare_and_notify(&boxrec_odds, bout, &25f32);
    }

    // Save cache after running
    if let Some(cache_path) = &config.cache_path {
        let serialised = serde_yaml::to_string(&boxers.values().collect::<Vec<_>>())?;
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(cache_path)?;
        file.write(
            serde_yaml::to_string(
                &boxers.values()
                    .collect::<Vec<_>>()
            )?.as_bytes())?;
    }

    //config.save()?;
    Ok(())
}
