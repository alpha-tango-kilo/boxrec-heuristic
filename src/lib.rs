use std::fs;
use std::error::Error;
use std::collections::HashMap;

use scraper::{Html, Selector};
use serde::Deserialize;
use regex::Regex;

const DATA_DIR: &str = "./data";

pub struct Args {
    pub name_one: String,
    pub name_two: String,
}

impl Args {
    pub fn new(mut args: std::env::Args) -> Result<Args, Box<dyn Error>> {
        args.next();

        Ok(Args {
            name_one: args.next()
                .ok_or("Missing boxers' names")?,
            name_two: args.next()
                .ok_or("Missing second boxer's name")?,
        })
    }
}

#[derive(Deserialize)]
struct Config {
    cache_path: Option<String>,
    weightings: Weightings,
}

impl Config {
    fn new(path: &str) -> Config {
        match fs::read_to_string(path) {
            Ok(contents) => match toml::from_str(contents.as_str()) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("Failed to parse config file, using default (Error: {})", err);
                    Config::new_default()
                }
            },
            Err(err) => {
                eprintln!("Failed to read config file, using default (Error: {})", err);
                Config::new_default()
            },
        }
    }

    fn new_default() -> Config {
        // Cache by default
        Config {
            cache_path: Some("./cache.csv".to_string()),
            weightings: Weightings {},
        }
    }
}

#[derive(Deserialize)]
struct Weightings {
    // TODO
}

pub fn generate_name_cache(cache_path_opt: &Option<String>) -> Result<HashMap<String, u32>, Box<dyn Error>> {
    let mut name_id_map: HashMap<String, u32> = HashMap::new();
    let mut success_count: u32 = 0; // temporary
    let mut failure_count: u32 = 0;
    let selector = Selector::parse("title").unwrap();
    let id_no_re = Regex::new(r"[0-9]{3,}\.htm$").unwrap();

    let paths = fs::read_dir(DATA_DIR)?
        // Read only files with permissions
        .filter_map(|rde| {
            // Convert to Option
            rde.ok()
                .and_then(|de| {
                    // Get path and check extension
                    de.path()
                        .extension()
                        .and_then(|ext| {
                            // Keep HTML only
                            // TODO: check case sensitivity
                            if ext == "htm" {
                                // Read file to string
                                de.path()
                                    .into_os_string()
                                    .into_string()
                                    .ok()
                            } else {
                                // Incorrect extension
                                None
                            }
                        })
                })
        });

    // Allocations outside the loop
    let mut file_name;
    let mut id: u32;
    let mut read_result;

    // For each path
    for path in paths {
        if success_count > 100 {
            break;
        }
        // Extract name and extension "1234.htm"
        file_name = id_no_re.find(&path.as_str())
            .unwrap() // panics if regex isn't matched
            .as_str();
        // Removes ".htm" ending from the file name
        id = file_name[..&file_name.len() - 4].parse()
            // Panics if the regex matched a non-number
            .unwrap();
        // Read & parse HTML
        // Returns Err if any file fails to be read
        // Because at this point it should be readable I believe, given we worked over the DirEntries
        read_result = fs::read_to_string(&path);
        if read_result.is_err() {
            eprintln!("Failed to read {}", file_name);
            failure_count += 1;
            continue;
        }
        // Parse HTML
        match Html::parse_document(
            read_result.unwrap()
                .as_str()
        )
            // Find h1 tags
            .select(&selector)
            // Get what's contained in the h1 tags
            .map(|er| er.inner_html())
            // The correct tag is always in the form <title>
            .find(|s| s.starts_with("BoxRec: "))
            // TODO: Convert HTML escape sequences to characters
        { // Match the find Option result
            // If you get a name
            Some(name) => {
                // Insert it into the HashMap against the ID
                // Maybe check if something is returned, given this indicates a duplicate name
                // If there are duplicates, don't store anything maybe?
                name_id_map.insert(name[8..].to_string(),id);
                success_count += 1; // temporary
            },
            // Print error if no name found in file, but carry on
            None => {
                eprintln!("Unable to find name for {}", file_name);
                failure_count += 1;
            },
        };
    }

    println!("{:#?}", name_id_map);
    println!("Success: {}\nFailure: {}", name_id_map.len(), failure_count);

    // Write name_id_map out to a CSV file in the same directory as the executable
    if let Some(cache_path) = cache_path_opt {
        println!("Writing Name -> ID map to {}", cache_path);
        let mut writer = csv::Writer::from_path(cache_path)?;
        name_id_map.iter()
            .for_each(|(k, v)| {
                if let Err(gah) = writer.write_record(&[k, &v.to_string()]) {
                    eprintln!("Failed to serialise {} => {}, skipping (Error: {})", k, v, gah);
                }
            });
        // CSV writers maintain an internal buffer, so it's important to flush when done
        writer.flush()?;
    }

    Ok(name_id_map)
}

pub fn read_name_cache(path: &str) -> Result<HashMap<String, u32>, Box<dyn Error>> {
    let mut reader = csv::Reader::from_path(path)?;
    let mut name_id_map: HashMap<String, u32> = HashMap::new();

    reader.records()
        .for_each(|result| {
            // Serialise and silently discard failures (borked)
            /*result.and_then(|record: csv::StringRecord| {
                record.deserialize(None)
                    .and_then(|(name, id)| {
                        name_id_map.insert(name, id);
                    })
            */
            // Noisily discard failures
            match result {
                Ok(record) => {
                    match record.deserialize(None) {
                        Ok((name, id)) => { name_id_map.insert(name, id); },
                        Err(err) => eprintln!("Failed to deserialise record (Error: {})", err),
                    }
                },
                Err(err) => eprintln!("Failed to read record (Error: {})", err),
            }
        });
    //println!("{:#?}", name_id_map);
    Ok(name_id_map)
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    // TODO: make this changeable using a flag
    let config = Config::new("./config.toml");

    // If caching is enabled, do things here
    if let Some(cache_path) = &config.cache_path {
        //read_name_cache(&cache_path)?;
    }

    //generate_name_cache(&config.cache_path)?;

    Ok(())
}
