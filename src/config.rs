use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const CONFIG_PATH: &str = "./config.yml";

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub cache_path: Option<String>,     // directory path to store cache files
    pub username: Option<String>,       // username for BoxRec
    pub password: Option<String>,       // password for BoxRec
    request_delay: Option<u64>,         // minimum time between BoxRec requests
    notify_threshold: Option<f32>,      // positive difference in our odds required to get notified
    warning_threshold: Option<f32>,     // if either boxer's BoxRec score is below this, warn user
    recheck_delay: Option<u16>,         // time in minutes between Betfair checks
}

impl Config {
    pub fn new(path: &str) -> Config {
        match fs::read_to_string(path) {
            Ok(contents) => match serde_yaml::from_str::<Config>(contents.as_str()) {
                Ok(config) => {
                    // Validate numbers
                    if let Some(percent) = &config.notify_threshold {
                        if percent < &0f32 || percent > &100f32 {
                            eprintln!("Config had bad notify_threshold, using default configuration (Read: {}%)", percent);
                            return Config::new_default();
                        }
                    }
                    config
                },
                Err(why) => {
                    eprintln!("Failed to parse config file, using default (Error: {})", why);
                    Config::new_default()
                },
            },
            Err(why) => {
                eprintln!("Failed to read config file, using default (Error: {})", why);
                Config::new_default()
            },
        }
    }

    pub fn new_default() -> Config {
        // Sensible defaults™
        Config {
            cache_path: Some(String::from("./.cache")), // Cache by default
            username: None,
            password: None,
            request_delay: Some(500u64),
            notify_threshold: Some(15f32),
            warning_threshold: Some(2f32),
            recheck_delay: Some(60u16),
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let ser = serde_yaml::to_string(&self)?;
        match OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(CONFIG_PATH)?
            .write_all(ser.as_bytes())
        {
            Ok(_) => Ok(()),
            Err(why) => {
                eprintln!("Failed to save config file (Error: {})", why);
                eprintln!("Here's the config if you wanted it:\n{}", ser);
                Err(why.into())
            },
        }
    }

    pub fn get_request_delay(&self) -> Duration {
        let ms = match self.request_delay {
            Some(ms) => ms,
            None => Config::new_default().request_delay.unwrap(),
        };
        Duration::from_millis(ms)
    }

    pub fn get_notify_threshold(&self) -> f32 {
        match self.notify_threshold {
            Some(percent) => percent,
            None => Config::new_default().notify_threshold.unwrap(),
        }
    }

    pub fn get_warning_threshold(&self) -> f32 {
        self.warning_threshold.unwrap_or(Config::new_default().warning_threshold.unwrap())
    }

    pub fn get_recheck_delay(&self) -> Duration {
        let mins = match self.recheck_delay {
            Some(mins) => mins,
            None => Config::new_default().recheck_delay.unwrap(),
        };
        Duration::from_secs((mins * 60) as u64)
    }
}
