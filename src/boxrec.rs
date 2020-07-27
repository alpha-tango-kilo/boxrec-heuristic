use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};

use regex::Regex;
use reqwest::blocking::{Client, Response};
use scraper::{Html, Selector};
use trim_in_place::TrimInPlace;

use crate::Config;

struct Login {
    username: String,
    password: String,
}

impl Login {
    // Maybe find some way to do this without cloning?
    fn get(config: &Config) -> Result<Login, Box<dyn Error>> {
        let username;
        let password;

        match &config.username {
            Some(name) => {
                username = name.clone();
                match &config.password {
                    Some(pwd) => password = pwd.clone(),
                    None => password = Login::take_from_user("Enter password: ")?,
                }
            },
            None => {
                username = Login::take_from_user("Enter username: ")?;
                // If username not specified, then always take password
                password = Login::take_from_user("Enter password: ")?;
            },
        };

        Ok(Login { username, password })
    }

    fn take_from_user(prompt: &str) -> Result<String, Box<dyn Error>> {
        let mut input = String::new();
        print!("{}", prompt);
        // ensures the prompt is actually printed, as Rust usually only flushes on newline
        io::stdout().flush()?;
        io::stdin()
            .read_line(&mut input)?;
        // I actually had to get a crate for this...
        input.trim_in_place();
        Ok(input)
    }
}

pub fn init() -> Result<Client, reqwest::Error> {
    // Basic synchronous client with cookies enabled
    reqwest::blocking::Client::builder()
        .cookie_store(true)
        .build()
}

pub fn login(config: &Config, client: &Client) -> Result<(), Box<dyn Error>> {
    let login = Login::get(config)?;

    let mut form_data = HashMap::new();
    form_data.insert("_username", login.username.as_str());
    form_data.insert("_password", login.password.as_str());
    form_data.insert("_remember_me", "on");
    form_data.insert("login[go]", "");

    println!("Sending login request");

    let send = client.post("https://boxrec.com/en/login")
        .form(&form_data);

    //println!("Request: {:#?}", send);

    let response = send.send()?;

    //println!("Response: {:#?}", response);

    // TODO: When reqwest supports it, check cookies set instead of making an extra request to check if login was successful
    let req = client.get("https://boxrec.com/en/my_details").send()?;
    //println!("Response: {:#?}", req);

    if am_beaned(&req) {
        Err("Failed to login".into())
    } else {
        println!("Logged in successfully");
        Ok(())
    }
}

fn am_beaned(response: &Response) -> bool {
    response.url().as_str().contains("login")
}

pub fn get_page_by_id(client: &Client, id: u32) -> Result<Html, Box<dyn Error>> {
    let url = format!("https://boxrec.com/en/proboxer/{}", id);
    let req = client.get(&url).send()?;

    if am_beaned(&req) {
        Err("Logged out by BoxRec".into())
    } else {
        let req_text = req.text()?;
        //println!("{}", req_text);
        Ok(Html::parse_document(req_text.as_str()))
    }
}

pub fn boxer_search(client: &Client, forename: &str, surname: &str, active_only: bool) -> Result<u32, Box<dyn Error>> {
    // Step 1: perform request
    let forename = forename.to_lowercase();
    let surname = surname.to_lowercase();
    let url = format!(
        "https://boxrec.com/en/search?p[first_name]={}&p[last_name]={}&p[role]=fighters&p[status]={}&pf_go=go&p[orderBy]=&p[orderDir]=ASC",
        forename,
        surname,
        if active_only { "a" } else { "" }
    );
    let req = client.get(&url).send()?;

    if am_beaned(&req) {
        return Err("Logged out by BoxRec".into());
    }

    // Step 2: parse results
    let req = req.text()?;
    let req = Html::parse_document(req.as_str());
    let selector = Selector::parse("a.personLink").unwrap();
    let mut results = req.select(&selector).peekable();
    let re = Regex::new(r"[0-9]{3,}").unwrap();

    let target;
    let search_in;
    let choices;

    if results.peek().is_none() {
        // Error if there are no results
        return Err("No results".into());
    } else if results.peek().unwrap().inner_html().to_lowercase() == format!("{} {}", forename, surname) {
        // Exact match, accept
        search_in = results.next()
            .unwrap()
            .html();
        // Find ID of boxer using regex search
        target = re.find(search_in.as_str()).unwrap().as_str();
    } else {
        // No exact match, list results and have user pick
        println!("Exact match not found. Please choose your fighter");
        choices = results.enumerate()
            .map(|(n, er)| -> String {
                println!("{}) {}", n+1, er.inner_html());
                er.html()
            })
            .collect::<Vec<_>>();
        // Handle user input
        let choice: usize;
        loop {
            print!("Pick a number: ");
            io::stdout().flush()?;
            let mut temp = String::new();
            io::stdin()
                .read_line(&mut temp)?;
            match temp.trim().parse::<usize>() {
                Ok(n) => {
                    if n > 0 && n < choices.len() {
                        // Account for offset
                        choice = n - 1;
                        break;
                    } else {
                        println!("Please pick a valid number");
                    }
                },
                Err(_) => println!("No, actually pick a number"),
            }
        }
        // End user input
        // Find ID of boxer using regex search
        target = re.find(
            choices.get(choice).unwrap()
        ).unwrap()
            .as_str();
    }
    // Parse String ID of boxer to u32
    let boxer_id = target.parse::<u32>().unwrap();
    println!("Selected: {}", boxer_id);
    Ok(boxer_id)
}

pub fn get_boxer_page(client: &Client, id: &u32) -> Result<Html, Box<dyn Error>> {
    let url = format!("https://boxrec.com/en/proboxer/{}", id);
    let response = client.get(&url).send()?;
    let content = response.text()?;
    Ok(Html::parse_document(&content))
}

// TODO: maybe make args a bit more user friendly
pub fn find_upcoming_bout(client: &Client, id_1: &u32, name_2: &str) -> Result<Html, Box<dyn Error>> {
    let boxer_1 = get_boxer_page(client, id_1)?;
    let name_2 = name_2.to_lowercase();
    let scheduled_bouts_selector = Selector::parse(".scheduleRow").unwrap();

    let mut scheduled_fights = boxer_1.select(&scheduled_bouts_selector).peekable();

    if scheduled_fights.peek().is_none() {
        return Err(format!("Boxer {} has no scheduled fights", id_1).into()); // TODO: return name instead
    }

    let bout_link_regex = Regex::new(r"/en/event/[0-9]{6,}/[0-9]{7,}").unwrap();

    for upcoming_fight in scheduled_fights {
        let upcoming_fight = upcoming_fight.html();
        // Check if a URL is found first, this isn't guaranteed
        match bout_link_regex.find(&upcoming_fight) {
            // If a URL is found, check that this entry is for the correct opponent
            Some(link) => if upcoming_fight.to_lowercase().contains(&name_2) {
                println!("Found matching bout");
                // Once a matching bout has been found, download the page
                let url = format!("https://boxrec.com{}", link.as_str());
                let bout_page = client.get(&url).send()?.text()?;
                return Ok(Html::parse_document(&bout_page));
            },
            None => {},
        }
    }
    // If nothing is found after going through all the scheduled entries, say we couldn't find any
    Err("Unable to find any bouts matching search criteria".into())
}
