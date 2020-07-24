use std::collections::HashMap;
use std::error::Error;
use std::io;

use reqwest::blocking::Client;

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
        io::stdin()
            .read_line(&mut input)?;
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

    println!("Response: {:#?}", response);

    // Note the response code doesn't actually dictate if you were logged in successfully
    // TODO: Identify if login was successful

    Ok(())
}