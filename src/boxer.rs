use std::error::Error;

use regex::Regex;
use scraper::Selector;
use serde::{Deserialize, Serialize};

use crate::boxrec::BoxRecAPI;

#[derive(Debug, Serialize, Deserialize)]
pub struct Boxer {
    id: u32,
    forename: String,
    surname: String,
}

impl Boxer {
    pub fn new_by_name(api: &BoxRecAPI, name: String) -> Option<Boxer> {
        let (forename, surname) = match split_name(&name) {
            Ok(tup) => tup,
            Err(err) => {
                eprintln!("{}", err);
                return None;
            },
        };
        match api.boxer_search(&forename, &surname, false) {
            Ok(id) => Some(Boxer {
                id,
                forename,
                surname,
            }),
            Err(err) => {
                eprintln!("Failed to get boxer \"{}\" (Error: {})", name, err);
                None
            },
        }
    }

    pub fn new_by_id(api: &BoxRecAPI, id: u32) -> Option<Boxer> {
        let page = match api.get_boxer_page_by_id(&id) {
            Ok(page) => page,
            Err(err) => {
                eprintln!("Unable to find boxer {} (Error: {})", id, err);
                return None;
            },
        };

        let title_tag_selector: Selector = Selector::parse("title").unwrap();

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

    pub fn get_id(&self) -> &u32 { &self.id }

    pub fn get_bout_scores(&self, api: &BoxRecAPI, opponent: &Boxer) -> Result<(f32, f32), Box<dyn Error>> {
        let bout_page = api.get_bout_page(&self.id, &opponent.get_name())?;
        let table_row_selector = Selector::parse(".responseLessDataTable").unwrap();
        let float_regex = Regex::new(r"[0-9]+\.[0-9]+").unwrap();

        for row in bout_page.select(&table_row_selector) {
            let raw_html = row.html();
            if raw_html.contains("after fight") {
                let scores = float_regex.find_iter(&raw_html)
                    .filter_map(|m| -> Option<f32> {
                        // Take the snip identified by the regex
                        raw_html[m.start()..m.end()]
                            // Parse it as a float
                            .parse::<f32>()
                            // And convert it to an option so the filter_map drops all the bad ones
                            .ok()
                    })
                    // Aggressively shove this into a vector
                    .collect::<Vec<_>>();
                return if scores.len() != 2 {
                    // What the fonk did the regex match?!
                    Err(format!("Didn't find two scores, confused. (Found: {:?})", scores).into())
                } else {
                    Ok((
                        *scores.get(0).unwrap(),
                        *scores.get(1).unwrap(),
                    ))
                };
            }
        }
        Err("Couldn't find scores on bout page".into())
    }
}

fn split_name(name: &str) -> Result<(String, String), &'static str> {
    // Takes first word as forename and the rest as surname
    match name.find(" ") {
        Some(index) => Ok((
            String::from(&name[..index]),
            String::from(&name[index..]),
        )),
        None => Err("Malformed name: no spaces")
    }
}
