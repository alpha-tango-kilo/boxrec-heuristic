use std::fs;
use std::error::Error;

pub struct Config {
    pub name_one: String,
    pub name_two: String,
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, Box<dyn Error>> {
        args.next();

        Ok(Config {
            name_one: "Whee".to_string(),
            name_two: "Wooo".to_string(),
        })
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    Ok(())
}
