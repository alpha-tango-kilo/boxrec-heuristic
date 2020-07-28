use scraper::Selector;
use serde::{Deserialize, Serialize};

use crate::boxrec::BoxRecAPI;

#[derive(Debug, Serialize, Deserialize)]
pub struct Boxer {
    pub id: u32,
    pub name: String,
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
                let name = String::from(&name[8..]);
                Some(Boxer {
                    id,
                    name,
                })
            },
            None => {
                eprintln!("Unable to find name in {}'s page", id);
                None
            },
        }
    }
}