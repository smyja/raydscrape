import requests
import json
import time
import os

# Set the base URL and headers
url = 'https://api-v3.raydium.io/pools/info/list'
headers = {
    'accept': 'application/json, text/plain, */*',
    'accept-language': 'en-US,en;q=0.9',
    'dnt': '1',
    'if-none-match': 'W/"305ae-4ZutJwbjkMKhcxXJZZaAfVLNJVA"',
    'origin': 'https://raydium.io',
    'priority': 'u=1, i',
    'referer': 'https://raydium.io/',
    'sec-ch-ua': '"Not-A.Brand";v="99", "Chromium";v="124"',
    'sec-ch-ua-mobile': '?0',
    'sec-ch-ua-platform': '"macOS"',
    'sec-fetch-dest': 'empty',
    'sec-fetch-mode': 'cors',
    'sec-fetch-site': 'same-site',
    'user-agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'
}

# Parameters for the API call
params = {
    'poolType': 'all',
    'poolSortField': 'default',
    'sortType': 'desc',
    'pageSize': 100,
    'page': 1
}

# File paths for checkpoint and output data
checkpoint_file = 'checkpoint.txt'
data_file = 'raydium_pools_data.json'
max_retries = 5  # Maximum number of retries if we hit a rate limit or error
delay_between_pages = 2  # Delay between each page request in seconds

# Helper function to load the checkpoint
def load_checkpoint():
    if os.path.exists(checkpoint_file):
        with open(checkpoint_file, 'r') as f:
            return int(f.read().strip())
    return 1  # Start from page 1 if no checkpoint exists

# Helper function to save the checkpoint
def save_checkpoint(page_number):
    with open(checkpoint_file, 'w') as f:
        f.write(str(page_number))

# Helper function to load the existing data
def load_existing_data():
    if os.path.exists(data_file):
        with open(data_file, 'r') as f:
            return json.load(f)
    return []

# Initialize with existing data (if any)
all_data = load_existing_data()

# Load the last page processed from the checkpoint
params['page'] = load_checkpoint()

while True:
    for retry in range(max_retries):
        try:
            # Make the API request
            response = requests.get(url, headers=headers, params=params)

            # Check if the request was successful
            if response.status_code == 200:
                parsed_data = response.json()
                # Example of properly accessing fields in the response.
           
                data = parsed_data['data']['data']  # Drill into 'data' field if response has it.
                

                # If no data is returned, we have reached the last page
                if not data:
                    print(f"Reached the last page at page {params['page']}. No more data.")
                    break
                
                # Append the data to the list
                all_data.extend(data)

                # Append data to the file immediately (optional, helps avoid losing data)
                with open(data_file, 'w') as f:
                    json.dump(all_data, f, indent=4)

                # Save the current page number as a checkpoint
                save_checkpoint(params['page'])

                # Move to the next page
                print(f"Fetched page {params['page']}")
                params['page'] += 1

                # Respectful delay between requests to avoid rate-limiting
                time.sleep(delay_between_pages)

                # Break out of the retry loop since the request succeeded
                break
            elif response.status_code == 429:
                # Handle rate limit: Retry after waiting for the specified time in the `Retry-After` header
                retry_after = int(response.headers.get("Retry-After", 5))  # Default to 5 seconds if header is missing
                print(f"Rate limited. Retrying after {retry_after} seconds.")
                time.sleep(retry_after)
            else:
                print(f"Unexpected status code {response.status_code}: {response.text}")
                break
        except requests.exceptions.RequestException as e:
            print(f"Request failed: {e}")
            if retry < max_retries - 1:
                # Wait before retrying
                wait_time = 2 ** retry  # Exponential backoff (2, 4, 8, ...)
                print(f"Retrying in {wait_time} seconds...")
                time.sleep(wait_time)
            else:
                print(f"Max retries reached. Skipping page {params['page']}.")
                break

# Final save of all data after finishing
with open(data_file, 'w') as f:
    json.dump(all_data, f, indent=4)

print(f"Data saved to '{data_file}'.")
