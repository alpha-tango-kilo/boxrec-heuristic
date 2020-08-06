use std::error::Error;

use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Bout {
    pub fighter_one: String,
    pub fighter_two: String,
    pub odds: BoutOdds,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoutOdds {
    pub one_wins: Odds,
    pub draw: Odds,
    pub two_wins: Odds,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Odds {
    top: u32,
    bottom: u32,
}

// https://www.aceodds.com/bet-calculator/odds-converter.html
impl Odds {
    fn from_mangled_string(s: String) -> Result<Odds, Box<dyn Error>> {
        // Warning: copious jank
        // The only way to solve this would be to match the exact <span> element that holds the fraction
        // But its CSS is no different to the parent <a> element, so I end up getting the whole <span> element instead of the inner HTML of it, which would be just the fraction
        // The arg typically looks something like "\n<span class=\"ui-runner-price ui-924_231809773-28625857 ui-display-fraction-price\">\n8/15\n<", "span>\n"

        // Edge case - 'evens' odds
        if s.contains("EVS") {
            Ok(Odds {
                top: 1,
                bottom: 1,
            })
        } else {
            // Split on /
            let mut parts = s.split("/");

            Ok(Odds {
                top: parts.next()
                    // Check we actually got something - woo safety!
                    .ok_or("Top of fraction not found")?
                    // Reverse split because for the top number it's at the end of the string
                    .rsplit("\n")
                    // The number is the first thing as it's always after a \n (see above sample)
                    .next()
                    // Unwrapping this option can never fail because a split always returns an iterator of at least one item
                    .unwrap()
                    // Make it a u32!
                    .parse()?,

                bottom: parts.next()
                    .ok_or("Bottom of fraction not found")?
                    // Normal split in this case as the number should be the first item before a \n (see above sample)
                    .split("\n")
                    .next()
                    .unwrap()
                    .parse()?,
            })
        }
    }

    // Used to quote profit
    pub fn as_frac(&self) -> String {
        format!("{}/{}", self.top, self.bottom)
    }

    // Used to show bookie's perceived odds
    pub fn as_percent(&self) -> f32 {
        100f32 * (1f32 - self.top as f32 / self.bottom as f32)
    }

    // Used to calculate return (profit + stake)
    pub fn as_decimal(&self) -> f32 {
        1f32 + self.top as f32 / self.bottom as f32
    }
}

pub struct BetfairAPI {
    reqwest_client: Client,
}

impl BetfairAPI {
    pub fn new() -> Result<BetfairAPI, reqwest::Error> {
        // Synchronous client, no cookies
        Ok(BetfairAPI {
            reqwest_client:
                Client::builder().build()?
        })
    }

    pub fn get_listed_bouts(&self) -> Result<Vec<Bout>, Box<dyn Error>> {
        let page = Html::parse_document(
            &self.reqwest_client
                .get("https://www.betfair.com/sport/boxing")
                .send()?
                .text()?
        );

        let bout_selector = Selector::parse(".com-coupon-line-new-layout.avb-table.quarter-template.avb-row").unwrap();

        println!("Checking bouts");
        Ok(
            page.select(&bout_selector)
                .filter_map(|er| -> Option<Bout> {
                    let odds = get_bout_odds(&er);
                    match odds {
                        Ok(odds) => {
                            match get_bout_names(&er) {
                                Ok((fighter_one, fighter_two)) => Some(Bout { fighter_one, fighter_two, odds }),
                                Err(err) => {
                                    eprintln!("Failed to get names for a bout (Error: {})", err);
                                    None
                                }
                            }

                        },
                        Err(err) => {
                            eprintln!("Failed to get odds for a bout (Error: {})", err);
                            None
                        },
                    }
                })
                .collect()
        )
    }
}

fn get_bout_names(fragment: &ElementRef) -> Result<(String, String), Box<dyn Error>> {
    let name_selector = Selector::parse(".team-name").unwrap();
    let mut names = fragment.select(&name_selector)
        .map(|er2| -> String {
            String::from(
                er2.inner_html().trim()
            )
        });

    Ok((
        names.next().ok_or("First fighter's name not found")?,
        names.next().ok_or("Second fighter's name not found")?,
    ))
}

fn get_bout_odds(fragment: &ElementRef) -> Result<BoutOdds, Box<dyn Error>> {
    let odds_button_selector = Selector::parse(".com-bet-button").unwrap();
    let mut raw_fracs = fragment.select(&odds_button_selector)
        .map(|er| { er.inner_html() })
        //.map(|s| { println!("{}", s); s })
        .map(Odds::from_mangled_string);

    Ok(BoutOdds {
        one_wins: raw_fracs.next().unwrap()?,
        draw:     raw_fracs.next().unwrap()?,
        two_wins: raw_fracs.next().unwrap()?,
    })
}
