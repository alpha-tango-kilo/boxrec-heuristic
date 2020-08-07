use std::error::Error;
use std::io::{self, Write};
use std::ops::Sub;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use regex::Regex;
use reqwest::blocking::{Client, RequestBuilder};
use scraper::{Html, Selector};
use trim_in_place::TrimInPlace;

use crate::Config;

struct Login {
    username: String,
    password: String,
}

impl Login {
    // Maybe find some way to do this without cloning?
    fn get_from_config(config: &Config) -> Result<Login, Box<dyn Error>> {
        let username;
        let password;

        match &config.username {
            Some(name) => {
                username = name.clone();
                match &config.password {
                    Some(pwd) => password = pwd.clone(),
                    None => password = take_from_user("Enter password: ")?,
                };
            },
            None => {
                username = take_from_user("Enter username: ")?;
                // If username not specified, then always take password
                password = take_from_user("Enter password: ")?;
            },
        };

        Ok(Login { username, password })
    }
}

pub struct BoxRecAPI {
    reqwest_client: Client,
    request_delay: Duration,
    last_sent: SystemTime,
    login: Login,
}

impl BoxRecAPI {
    pub fn new(config: &Config) -> Result<BoxRecAPI, Box<dyn Error>> {
        // Basic synchronous client with cookies enabled
        let request_delay = Duration::from_millis(config.get_request_delay());
        Ok(BoxRecAPI {
            reqwest_client:
                Client::builder()
                    .cookie_store(true)
                    .build()?,
            request_delay,
            last_sent: SystemTime::now().sub(request_delay),
            login: Login::get_from_config(config)?,
        })
    }

    // Returns a reference to self to allow for chaining
    fn wait_if_needed(&mut self) -> &Self {
        let time_since_request = SystemTime::now()
            .duration_since(self.last_sent)
            .unwrap();
        // If we need to wait, do
        if time_since_request.lt(&self.request_delay) {
            // Calculate the time until we're okay to make the next request, and sleep for that time
            sleep(self.request_delay.sub(time_since_request));
        }
        // Update last sent time
        self.last_sent = SystemTime::now();
        self
    }

    pub fn login(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Sending login request");

        let response = self.wait_if_needed().reqwest_client
            .post("https://boxrec.com/en/login")
            .form::<[(&str, &str); 4]>(&[
                ("_username", &self.login.username),
                ("_password", &self.login.password),
                ("_remember_me", "on"),
                ("login[go]", ""),
            ])
            .send()?;

        // If login is successful, you are redirected to the home page instead of the login page
        if response.url().as_str() == "https://boxrec.com/en/login" {
            Err("Failed to login".into())
        } else {
            println!("Logged in successfully");
            Ok(())
        }
    }

    fn try_request_and_unwrap(&mut self, req: &RequestBuilder) -> Result<String, Box<dyn Error>> {
        loop {
            self.wait_if_needed();
            let response = req.try_clone().ok_or("Failed to clone request")?.send()?;
            if response.url().as_str().contains("login") {
                eprintln!("Logged out by BoxRec, attempting to login");
                self.login()?;
            } else {
                let text = response.text()?;
                if text.contains("Please complete the form below to continue...") {
                    println!("BoxRec is prompting for a reCAPTCHA\n\
                    Please visit the website and complete one under the login used \
                    by this program");
                    loop {
                        if take_from_user("Once done, type 'go': ")?.to_lowercase() == "go" {
                            break;
                        }
                    }
                } else {
                    return Ok(text);
                }
            }
        }
    }

    pub fn get_boxer_page_by_id(&mut self, id: &u32) -> Result<Html, Box<dyn Error>> {
        let url = format!("https://boxrec.com/en/proboxer/{}", id);
        let response = self.try_request_and_unwrap(&self.reqwest_client.get(&url))?;
        Ok(Html::parse_document(&response))
    }

    pub fn boxer_search(&mut self, forename: &str, surname: &str, active_only: bool) -> Result<u32, Box<dyn Error>> {
        // Step 1: perform request
        let forename = forename.to_lowercase();
        let surname = surname.to_lowercase();
        let url = format!(
            "https://boxrec.com/en/search?p[first_name]={}&p[last_name]={}&p[role]=fighters&p[status]={}&pf_go=go&p[orderBy]=&p[orderDir]=ASC",
            forename,
            surname,
            if active_only { "a" } else { "" }
        );
        let response = self.try_request_and_unwrap(&self.reqwest_client.get(&url))?;

        // Step 2: parse results
        let response = Html::parse_document(&response);
        let selector = Selector::parse("a.personLink").unwrap();
        let mut results = response.select(&selector).peekable();
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
            println!("Exact match not found for '{} {}'. Please choose your fighter",
                forename,
                surname);
            // Grab the name (inner_html) and the whole page
            choices = results.map(|er| (er.inner_html(), er.html()))
                .collect::<Vec<_>>();
            let choice: usize;
            if choices.len() > 1 {
                // If there are multiple options, pretty print them and have the user choose
                choices.iter()
                    .enumerate()
                    .for_each(|(n, (name, _))| {
                        println!("{}) {}", n + 1, name)
                    });
                // Handle user input
                loop {
                    print!("Pick a number: ");
                    io::stdout().flush()?;
                    let mut temp = String::new();
                    io::stdin()
                        .read_line(&mut temp)?;
                    match temp.trim().parse::<usize>() {
                        Ok(n) => {
                            if n > 0 && n <= choices.len() {
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
            } else {
                // If there's only one choice, pick the first item
                choice = 0;
            }
            // End user input
            // Find ID of boxer using regex search
            target = re.find(
                // Tuple index 1 to search the page contents
                &choices.get(choice).unwrap().1
            ).unwrap()
                .as_str();
        }
        // Parse String ID of boxer to u32
        let boxer_id = target.parse::<u32>().unwrap();
        println!("Selected: {}", boxer_id);
        Ok(boxer_id)
    }

    // TODO: maybe make args a bit more user friendly
    pub fn get_bout_page(&mut self, id_1: &u32, name_2: &str) -> Result<Html, Box<dyn Error>> {
        let boxer_1 = self.get_boxer_page_by_id(id_1)?;
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
                    let bout_page = self.try_request_and_unwrap(&self.reqwest_client.get(&url))?;
                    // Pass onto the next stage
                    return Ok(Html::parse_document(&bout_page));
                },
                None => {},
            }
        }
        // If nothing is found after going through all the scheduled entries, say we couldn't find any
        Err("Unable to find any bouts matching search criteria".into())
    }
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
