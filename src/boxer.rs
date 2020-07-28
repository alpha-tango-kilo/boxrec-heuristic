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
    pub fn get_by_name(api: &BoxRecAPI, name: &String) -> Option<Boxer> {
        // TODO
        None
    }

    pub fn get_by_id(api: &BoxRecAPI, id: u32) -> Option<Boxer> {
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
        { // Match the find Option result
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
