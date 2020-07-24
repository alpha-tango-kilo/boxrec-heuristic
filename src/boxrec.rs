use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};

use reqwest::blocking::{Client, Response};
use scraper::Html;
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
        return Err("Logged out by BoxRec".into());
    }
    let req_text = req.text()?;
    //println!("{}", req_text);
    Ok(Html::parse_document(req_text.as_str()))
}
