use std::collections::HashMap;
use std::error::Error;

use reqwest::blocking::Client;

use crate::Config;

pub fn init() -> Result<Client, reqwest::Error> {
    // Basic synchronous client with cookies enabled
    reqwest::blocking::Client::builder()
        .cookie_store(true)
        .build()
}

pub fn login(config: &Config, client: &Client) -> Result<(), Box<dyn Error>> {
    let mut form_data = HashMap::new();
    form_data.insert("_username", config.username.as_str());
    form_data.insert("_password", config.password.as_str());
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