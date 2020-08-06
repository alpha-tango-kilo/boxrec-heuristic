#![allow(dead_code)]

use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};

use serde::{Deserialize, Serialize};

use boxer::*;

use crate::betfair::{BetfairAPI, Bout};
use crate::boxrec::BoxRecAPI;

mod betfair;
mod boxer;
mod boxrec;

const CONFIG_PATH: &str = "./config.yaml";

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub cache_path: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    request_timeout: Option<u64>,
    notify_threshold: Option<f32>,
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
        // Sensible defaults™
        Config {
            cache_path: Some(String::from("./.cache")), // Cache by default
            username: None,
            password: None,
            request_timeout: Some(500),
            notify_threshold: Some(15f32),
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
            Err(err) => {
                eprintln!("Failed to save config file (Error: {})", err);
                eprintln!("Here's the config if you wanted it:\n{}", ser);
                Err(err.into())
            },
        }
    }

    fn get_request_timeout(&self) -> u64 {
        match &self.request_timeout {
            Some(ms) => *ms,
            None => Config::new_default().request_timeout.unwrap(),
        }
    }

    fn get_notify_threshold(&self) -> f32 {
        match &self.notify_threshold {
            Some(percent) => *percent,
            None => Config::new_default().notify_threshold.unwrap(),
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
    // Load config
    // TODO: make this changeable using a flag
    let config = Config::new(CONFIG_PATH);

    // Connect to BoxRec
    let mut boxrec = BoxRecAPI::new(config.get_request_timeout())?;
    boxrec.login(&config)?;

    // Connect to Betfair
    let betfair = BetfairAPI::new()?;
    // Scrape Betfair
    let bouts = betfair.get_listed_bouts()?;
    //println!("{:#?}", bouts);

    // Create Boxer HashMap (runtime cache/index of Boxers by name)
    let mut boxers: HashMap<String, Boxer> = HashMap::new();

    // Load disk cache before running
    if let Some(cache_path) = &config.cache_path {
        // Check for and create cache folder
        match fs::metadata(cache_path) {
            // If Ok(), it exists
            // If it's a file, get scared, otherwise, we have a folder!
            Ok(md) => if md.is_file() {
                return Err("Cache path points to an existing file".into());
            },
            Err(e) => match e.kind() {
                // If the folder doesn't exist yet, try and make it
                ErrorKind::NotFound => fs::create_dir_all(cache_path)?,
                // If there's another error be spooked
                _ => return Err(e.into()),
            }
        };
        // Read pre-existing boxers cache if present and in a good format
        match fs::read_to_string(format!("{}/boxers.yml", cache_path)) {
            Ok(serialised) => serde_yaml::from_str::<Vec<Boxer>>(&serialised)?
                    .iter()
                    .for_each(|b| { boxers.insert(b.get_name(), b.to_owned()); }),
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {},
                _ => return Err(err.into()),
            },
        };
        //println!("Read from disk cache into runtime index:\n{:#?}", boxers);
    }

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
                match Boxer::new_by_name(&mut boxrec, &bout.fighter_two) {
                    // If it works
                    Some(f2) => {
                        // Insert it into the map
                        boxers.insert(bout.fighter_two.to_string(), f2);
                        // Then get both references
                        (boxers.get(&bout.fighter_one).unwrap(),
                         boxers.get(&bout.fighter_two).unwrap())
                    },
                    // If it doesn't work, skip this bout
                    None => continue,
                }
            },
            // Same case as above, but other boxer is unknown
            (false, true) => {
                match Boxer::new_by_name(&mut boxrec, &bout.fighter_one) {
                    Some(f1) => {
                        boxers.insert(bout.fighter_one.to_string(), f1);
                        (boxers.get(&bout.fighter_one).unwrap(),
                         boxers.get(&bout.fighter_two).unwrap())
                    },
                    None => continue,
                }
            },
            // If neither boxer is known
            (false, false) => {
                // Try and make the first
                match Boxer::new_by_name(&mut boxrec, &bout.fighter_one) {
                    // If it worked, insert
                    Some(f1) => boxers.insert(bout.fighter_one.to_string(), f1),
                    // Skip this bout otherwise
                    None => continue,
                };
                match Boxer::new_by_name(&mut boxrec, &bout.fighter_two) {
                    // If it worked, insert
                    Some(f2) => boxers.insert(bout.fighter_two.to_string(), f2),
                    // Skip this bout if it didn't (at least we still have one more dude documented)
                    None => continue,
                };
                // Get both references
                (boxers.get(&bout.fighter_one).unwrap(),
                 boxers.get(&bout.fighter_two).unwrap())
            },
        };

        let boxrec_odds = match fighter_one.get_bout_scores(&mut boxrec, &fighter_two) {
            Ok(m) => m,
            Err(err) => {
                eprintln!("Failed to get matchup between {} & {} (Error: {})",
                          fighter_one.get_name(),
                          fighter_two.get_name(),
                          err);
                continue;
            },
        };
        compare_and_notify(&boxrec_odds, bout, &config.get_notify_threshold());
    }

    // Save disk cache after running
    if let Some(cache_path) = &config.cache_path {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(format!("{}/boxers.yml", cache_path))?;
        file.write(
            serde_yaml::to_string(
                &boxers.values()
                    .collect::<Vec<_>>()
            )?.as_bytes())?;
    }

    //config.save()?;
    Ok(())
}
