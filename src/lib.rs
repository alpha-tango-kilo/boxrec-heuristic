#![allow(dead_code)]

use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    ops::Deref,
    rc::Rc,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use BoutStatus::*;
use boxer::*;

use crate::{
    betfair::{BetfairAPI, Bout},
    boxrec::BoxRecAPI,
    config::{Config, CONFIG_PATH},
};

mod betfair;
pub mod boxer;
mod boxrec;
mod config;

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
            MissingBoxers => "Missing boxers",
            MissingBoutPage => "Missing bout page",
            Checked => "Odds compared between BoxRec & Betfair",
            Announced => "User notified of odds differential",
        })
    }
}

pub struct State {
    betfair: BetfairAPI,
    bout_metadata: Vec<BoutMetadata>,
    boxers: HashMap<String, Rc<Boxer>>,
    boxrec: BoxRecAPI,
    config: Config,
}

impl State {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Load config
        // TODO: make this changeable using a flag/arg
        let config = Config::new(CONFIG_PATH);

        // Connect to BoxRec
        let mut boxrec = BoxRecAPI::new(&config)?;
        boxrec.login()?;

        // Connect to Betfair
        let betfair = BetfairAPI::new()?;

        Ok(State {
            betfair,
            bout_metadata: Vec::new(),
            boxers: HashMap::new(),
            boxrec,
            config,
        })
    }

    pub fn task(&mut self) -> Result<Vec<Matchup>, Box<dyn Error>> {
        // Scrape Betfair
        let bouts = self.betfair.get_listed_bouts()?;
        //println!("{:#?}", bouts);
        let mut notifs = vec![];

        // Tag all bouts with metadata if they don't already have it
        bouts.into_iter()
            .for_each(|bout| {
                let bout = BoutMetadata(bout, MissingBoxers);
                if !self.bout_metadata.contains(&bout) { self.bout_metadata.push(bout); }
            });

        for BoutMetadata(bout, status) in self.bout_metadata.iter_mut() {
            // Step 1: Get boxers
            if status == &MissingBoxers {
                let mut have_one = true;
                let mut have_two = true;

                // If we don't have fighter one
                if !self.boxers.contains_key(&bout.fighter_one) {
                    // Look them up with BoxRec
                    if let Some(f1) = Boxer::new_by_name(&mut self.boxrec, &bout.fighter_one) {
                        // Insert them into the index if present
                        self.boxers.insert(bout.fighter_one.to_string(), Rc::new(f1));
                    } else {
                        have_one = false;
                    }
                }
                // If we don't have fighter two, same process as one
                if !self.boxers.contains_key(&bout.fighter_two) {
                    if let Some(f2) = Boxer::new_by_name(&mut self.boxrec, &bout.fighter_two) {
                        self.boxers.insert(bout.fighter_two.to_string(), Rc::new(f2));
                    } else {
                        have_two = false;
                    }
                }
                if have_one && have_two { status.next(); }
            }

            // Step 2: Get bout between boxers
            if status == &MissingBoutPage {
                let fighter_one = self.boxers.get(&bout.fighter_one).unwrap().clone();
                let fighter_two = self.boxers.get(&bout.fighter_two).unwrap().clone();

                let boxrec_odds = match Boxer::get_bout_scores(&self.config, &mut self.boxrec, fighter_one.clone(), fighter_two.clone()) {
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

                let threshold = self.config.get_notify_threshold();

                if boxrec_odds.win_percent_one - bout.odds.one_wins.as_percent() > threshold ||
                    boxrec_odds.win_percent_two - bout.odds.two_wins.as_percent() > threshold {
                    notifs.push(boxrec_odds);
                    status.next();
                }
                status.next();
            }
        }
        Ok(notifs)
    }

    pub fn read_cache(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(cache_path) = &self.config.cache_path.clone() {
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
                    .for_each(|b| { self.boxers.insert(b.get_name(), Rc::new(b)); }),
                Err(why) => match why.kind() {
                    ErrorKind::NotFound => {},
                    _ => return Err(why.into()),
                },
            };
            //println!("Read from disk cache into runtime index:\n{:#?}", boxers);

            // Read pre-existing bouts cache if present and in a good format
            match fs::read_to_string(format!("{}/bouts.yml", cache_path)) {
                Ok(serialised) => self.bout_metadata = serde_yaml::from_str::<Vec<BoutMetadata>>(&serialised)?,
                Err(why) => match why.kind() {
                    ErrorKind::NotFound => {},
                    _ => return Err(why.into()),
                },
            };
        }

        Ok(())
    }

    pub fn write_cache(&self) -> Result<(), Box<dyn Error>> {
        let cache_path = match &self.config.cache_path {
            Some(p) => p,
            None => return Ok(()),
        };

        let mut boxers_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(format!("{}/boxers.yml", cache_path))?;
        boxers_file.write(
            serde_yaml::to_string(
                &self.boxers.values()
                    .map(Rc::deref)
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
                &self.bout_metadata
            )?.as_bytes()
        )?;

        Ok(())
    }

    pub fn get_recheck_delay(&self) -> Duration {
        self.config.get_recheck_delay()
    }
}
