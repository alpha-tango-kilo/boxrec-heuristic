use reqwest::blocking::Client;

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
}
