use std::fmt::{self, Display};

use scraper::Selector;
use serde::{Deserialize, Serialize};

use crate::boxrec::BoxRecAPI;

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
}

impl Display for Boxer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (ID: {})", self.get_name(), self.id)
    }
}

impl PartialEq for Boxer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Boxer {}

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
