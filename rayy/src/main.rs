use reqwest::{Client, header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, DNT, ORIGIN, REFERER, USER_AGENT}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, thread, time};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    data: PoolData,
}

#[derive(Debug, Serialize, Deserialize)]
struct PoolData {
    data: Vec<PoolInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PoolInfo {
    #[serde(flatten)]
    details: Value,
}

// Helper functions remain unchanged
fn load_checkpoint(checkpoint_file: &str) -> u32 {
    if let Ok(content) = fs::read_to_string(checkpoint_file) {
        return content.trim().parse::<u32>().unwrap_or(1);
    }
    1 // Start from page 1 if no checkpoint exists
}

// Helper function to save the checkpoint (current page number)
fn save_checkpoint(checkpoint_file: &str, page_number: u32) {
    let mut file = fs::File::create(checkpoint_file).expect("Unable to open checkpoint file");
    writeln!(file, "{}", page_number).expect("Unable to write checkpoint");
}


// Update the load_existing_data and save_data functions
fn load_existing_data(data_file: &str) -> Vec<PoolInfo> {
    if let Ok(content) = fs::read_to_string(data_file) {
        serde_json::from_str(&content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    }
}

fn save_data(data_file: &str, all_data: &Vec<PoolInfo>) {
    let json_data = serde_json::to_string_pretty(all_data).expect("Unable to serialize data");
    fs::write(data_file, json_data).expect("Unable to write data file");
}
#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let url = "https://api-v3.raydium.io/pools/info/list";
    let checkpoint_file = "checkpoint.txt";
    let data_file = "raydium_pools_data.json";
    let max_retries = 5;
    let delay_between_pages = 2; // seconds

    // Set up headers for the request
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/plain, */*"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(DNT, HeaderValue::from_static("1"));
    headers.insert(ORIGIN, HeaderValue::from_static("https://raydium.io"));
    headers.insert(REFERER, HeaderValue::from_static("https://raydium.io/"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"));

    let client = Client::new();
    let mut all_data = load_existing_data(data_file);
    let mut current_page = load_checkpoint(checkpoint_file);

    // Main loop to keep fetching pages
    loop {
        let page_string = current_page.to_string();
        let params = vec![
            ("poolType", "all"),
            ("poolSortField", "default"),
            ("sortType", "desc"),
            ("pageSize", "100"),
            ("page", &page_string),
        ];

        let mut retries = 0;
        let mut success = false;

        while retries < max_retries {
            let request = client.get(url)
                .headers(headers.clone())
                .query(&params);

            let response = request.send().await;

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        let parsed_data: ApiResponse = res.json().await?;

                        if parsed_data.data.data.is_empty() {
                            println!("Reached the last page at page {}. No more data.", current_page);
                            return Ok(());
                        }

                        // Append the new data
                        all_data.extend(parsed_data.data.data);

                        // Save the current page's data immediately
                        save_data(data_file, &all_data);

                        // Save the checkpoint
                        save_checkpoint(checkpoint_file, current_page);

                        println!("Fetched page {}", current_page);

                        // Move to the next page
                        current_page += 1;

                        // Delay to avoid rate-limiting
                        thread::sleep(time::Duration::from_secs(delay_between_pages));

                        success = true;
                        break;
                    } else if res.status().as_u16() == 429 {
                        // Handle rate-limiting (status code 429)
                        let retry_after = res.headers()
                            .get("Retry-After")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(5);

                        println!("Rate limited. Retrying after {} seconds.", retry_after);
                        thread::sleep(time::Duration::from_secs(retry_after));
                    } else {
                        println!("Unexpected status code: {}", res.status());
                        break;
                    }
                }
                Err(err) => {
                    println!("Request failed: {}", err);
                    let wait_time = 2_u64.pow(retries); // Exponential backoff
                    println!("Retrying in {} seconds...", wait_time);
                    thread::sleep(time::Duration::from_secs(wait_time));
                    retries += 1;
                }
            }
        }

        if !success {
            println!("Max retries reached for page {}. Skipping.", current_page);
            current_page += 1;
        }
    }
}
