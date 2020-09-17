use std::{
    error::Error,
    fmt::{self, Display},
    rc::Rc,
};

use regex::Regex;
use scraper::Selector;
use serde::{Deserialize, Serialize};

use crate::boxrec::BoxRecAPI;
use crate::config::Config;

pub struct Matchup {
    pub fighter_one: Rc<Boxer>,
    pub fighter_two: Rc<Boxer>,
    pub win_percent_one: f32,
    pub win_percent_two: f32,
    pub warning: bool,
}

impl Matchup {
    fn new(config: &Config, fighter_one: Rc<Boxer>, fighter_one_score: f32, fighter_two: Rc<Boxer>, fighter_two_score: f32) -> Matchup {
        let win_percent_one = fighter_one_score / (fighter_one_score + fighter_two_score) * 100f32;
        Matchup {
            fighter_one,
            fighter_two,
            win_percent_one,
            win_percent_two: 100f32 - win_percent_one,
            warning: fighter_one_score + fighter_two_score < 2f32 * config.get_warning_threshold(),
        }
    }

    pub fn get_winner(&self) -> Rc<Boxer> {
        if self.win_percent_one > self.win_percent_two {
            self.fighter_one.clone()
        } else {
            self.fighter_two.clone()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Boxer {
    pub id: u32,
    pub forename: String,
    pub surname: String,
}

impl Boxer {
    pub fn new_by_name(api: &mut BoxRecAPI, name: &str) -> Option<Boxer> {
        let (forename, surname) = match split_name(&name) {
            Ok(tup) => tup,
            Err(why) => {
                eprintln!("{}", why);
                return None;
            },
        };
        match api.boxer_search(&forename, &surname, false) {
            Ok(id) => Some(Boxer {
                id,
                forename,
                surname,
            }),
            Err(why) => {
                eprintln!("Failed to get boxer \"{}\" (Error: {})", name, why);
                None
            },
        }
    }

    pub fn new_by_id(api: &mut BoxRecAPI, id: u32) -> Option<Boxer> {
        let page = match api.get_boxer_page_by_id(&id) {
            Ok(page) => page,
            Err(why) => {
                eprintln!("Unable to find boxer {} (Error: {})", id, why);
                return None;
            },
        };

        let title_tag_selector = Selector::parse("title").unwrap();

        match page
            // Find title tag(s)
            .select(&title_tag_selector)
            // Get what's contained in them
            .map(|er| er.inner_html())
            // The name is always in the form "BoxRec: Joe Bloggs"
            .find(|s| s.starts_with("BoxRec: "))
        { // Match the Option result
            Some(name) => {
                let (forename, surname) = split_name(&name[8..]).unwrap();
                Some(Boxer {
                    id,
                    forename,
                    surname,
                })
            },
            None => {
                eprintln!("Unable to find name in {}'s page", id);
                None
            },
        }
    }

    pub fn get_name(&self) -> String { format!("{} {}", self.forename, self.surname) }

    pub fn get_bout_scores(config: &Config, api: &mut BoxRecAPI, fighter_one: Rc<Boxer>, fighter_two: Rc<Boxer>) -> Result<Matchup, Box<dyn Error>> {
        let bout_page = api.get_bout_page(&fighter_one.id, &fighter_two.get_name())?;
        let table_row_selector = Selector::parse(".responseLessDataTable").unwrap();
        // Floats below 1 are written as .086 (of course they are), hence the * for the first number
        let float_regex = Regex::new(r"[0-9]*\.[0-9]+").unwrap();

        for row in bout_page.select(&table_row_selector) {
            let raw_html = row.html();
            if raw_html.contains("after fight") {
                let mut scores = float_regex.find_iter(&raw_html)
                    .filter_map(|m| -> Option<f32> {
                        // Take the snip identified by the regex
                        // Always add a zero to the start, just in case
                        format!("0{}", &raw_html[m.start()..m.end()])
                            // Parse it as a float
                            .parse::<f32>()
                            // And convert it to an option so the filter_map drops all the bad ones
                            .ok()
                    });
                return Ok(Matchup::new(
                    config,
                    fighter_one,
                    scores.next().ok_or("Couldn't find first fighter's score")?,
                    fighter_two,
                    scores.next().ok_or("Couldn't find second fighter's score")?,
                ));
            }
        }
        Err("Couldn't find scores on bout page".into())
    }
}

impl Display for Boxer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (ID: {})", self.get_name(), self.id)
    }
}

fn split_name(name: &str) -> Result<(String, String), String> {
    // Takes first word as forename and the rest as surname
    match name.find(" ") {
        Some(index) => Ok((
            String::from(name[..index].trim()),
            String::from(name[index..].trim()),
        )),
        None => Err(format!("Malformed name, no spaces in \"{}\"", name))
    }
}
