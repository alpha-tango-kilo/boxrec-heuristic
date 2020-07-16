use std::fs;
use std::error::Error;
use std::collections::HashMap;

use scraper::{Html, Selector};
use regex::Regex;

const DATA_DIR: &str = "./data";

pub struct Config {
    pub name_one: String,
    pub name_two: String,
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, Box<dyn Error>> {
        args.next();

        Ok(Config {
            name_one: args.next()
                .ok_or("Missing boxers' names")?
                .to_string(),
            name_two: args.next()
                .ok_or("Missing second boxer's name")?
                .to_string(),
        })
    }
}

pub fn generate_name_cache() -> Result<(), Box<dyn Error>> {
    let mut name_id_map: HashMap<String, u32> = HashMap::new();
    let selector = Selector::parse("h1").unwrap();
    let name_re = Regex::new(r"^\w+( \w+)+$").unwrap();
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

    // For each path
    for path in paths {
        // Extract name and extension "1234.htm"
        let file_name = id_no_re.find(&path.as_str())
            .unwrap() // panics if regex isn't matched
            .as_str();
        // Removes ".htm" ending from the file name
        let file_name: u32 = file_name[..&file_name.len() - 4].parse()
            // Panics if the regex matched a non-number
            .unwrap();
        // Read & parse HTML
        // Returns Err if any file fails to be read
        // Because at this point it should be readable I believe, given we worked over the DirEntries
        let html = Html::parse_document(fs::read_to_string(&path)?.as_str());
        // Find h1 tags
        match html.select(&selector)
            // Get what's contained in the h1 tags
            .map(|er| er.inner_html())
            // Match the contents against the name regex (first result returned)
            .find(|s| name_re.is_match(s))
        { // Match the find Option result
            // If you get a name
            Some(name) => {
                // Insert it into the HashMap against the ID
                // Maybe check if something is returned, given this indicates a duplicate name
                // If there are duplicates, don't store anything maybe?
                name_id_map.insert(name,file_name);
            },
            // Print error if no name found in file, but carry on
            None => eprintln!("Unable to find name for {}", file_name),
        };
    }

    println!("{:#?}", name_id_map);

    // Write name_id_map out to a CSV file in the same directory as the executable
    let mut writer = csv::Writer::from_path("./cache.csv")?;
    writer.write_record(name_id_map);
    // CSV writers maintain an internal buffer, so it's important to flush when done
    writer.flush()?;

    Ok(())
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    generate_name_cache()?;

    Ok(())
}
