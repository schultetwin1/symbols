use anyhow::Result;
use serde::{Deserialize, Serialize};
use strum::EnumString;

use std::io::Write;
use std::str::FromStr;

const GITHUB_APP_CLIENT_ID: &str = "16b64151aa0e7d4c31ec";

const GITHUB_DEVICE_LOGIN_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_DEVICE_LOGIN_CHECK_URL: &str = "https://github.com/login/oauth/access_token";

/// All the possible errors that could be returned by the login check URL. These
/// codes are documented here:
/// https://docs.github.com/en/developers/apps/building-oauth-apps/authorizing-oauth-apps#error-codes-for-the-device-flow
#[derive(EnumString, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum GitHubDeviceFlowErrorCode {
    /// This error occurs when the authorization request is pending and the user
    /// hasn't entered the user code yet. The app is expected to keep polling
    /// the POST https://github.com/login/oauth/access_token request without
    /// exceeding the interval, which requires a minimum number of seconds
    /// between each request.
    AuthorizationPending,
    ///  When you receive the slow_down error, 5 extra seconds are added to the
    ///  minimum interval or timeframe required between your requests using POST
    ///  https://github.com/login/oauth/access_token. For example, if the
    ///  starting interval required at least 5 seconds between requests and you
    ///  get a slow_down error response, you must now wait a minimum of 10
    ///  seconds before making a new request for an OAuth access token. The
    ///  error response includes the new interval that you must use.
    SlowDown,
    /// If the device code expired, then you will see the token_expired error.
    /// You must make a new request for a device code.
    ExpiredToken,
    /// The grant type must be urn:ietf:params:oauth:grant-type:device_code and
    /// included as an input parameter when you poll the OAuth token request
    /// POST https://github.com/login/oauth/access_token.
    UnsupportedGrantType,
    /// For the device flow, you must pass your app's client ID, which you can
    /// find on your app settings page. The client_secret is not needed for the
    /// device flow.
    IncorrectClientCredentials,
    /// The device_code provided is not valid.
    IncorrectDeviceCode,
    ///  When a user clicks cancel during the authorization process, you'll
    ///  receive a access_denied error and the user won't be able to use the
    ///  verification code again.
    AccessDenied,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubDeviceCodesRequest {
    /// The client ID you received from GitHub for your OAuth App.
    client_id: String,
    /// The scope of access the app needs. All the scopes can be found here:
    /// https://docs.github.com/en/developers/apps/building-oauth-apps/scopes-for-oauth-apps
    scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubDeviceCodesResponse {
    /// The device verification code is 40 characters and used to verify the
    /// device.
    device_code: String,
    /// The user verification code is displayed on the device so the user can
    /// enter the code in a browser. This code is 8 characters with a hyphen in
    /// the middle.
    user_code: String,
    /// The verification URL where users need to enter the user_code:
    /// https://github.com/login/device.
    verification_uri: String,
    /// The number of seconds before the device_code and user_code expire. The
    /// default is 900 seconds or 15 minutes.
    expires_in: u32,
    /// The minimum number of seconds that must pass before you can make a new
    /// access token request (POST https://github.com/login/oauth/access_token)
    /// to complete the device authorization. For example, if the interval is 5,
    /// then you cannot make a new request until 5 seconds pass. If you make
    /// more than one request over 5 seconds, then you will hit the rate limit
    /// and receive a slow_down error.
    interval: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubAccesCheckRequest {
    /// The client ID you received from GitHub for your OAuth App.
    client_id: String,
    /// The device verification code you received from the POST
    /// https://github.com/login/device/code request.
    device_code: String,
    /// The grant type must be urn:ietf:params:oauth:grant-type:device_code.
    grant_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubAccesCheckResponse {
    access_token: String,
    token_type: String,
    scope: String,
}

pub fn symbolserver_login() -> Result<()> {
    const SERVICE: &str = "com.symboserver.symbols";
    const USERNAME: &str = "symbolserver";
    let token = rpassword::prompt_password("Enter symbolserver.com API token: ")?;
    let entry = keyring::Entry::new(SERVICE, USERNAME)?;
    entry.set_password(&token)?;

    Ok(())
}

pub fn github_login() -> Result<()> {
    const SERVICE: &str = "com.symboserver.symbols";
    const USERNAME: &str = "github";
    let client = reqwest::blocking::Client::new();

    let codes = request_device_and_user_verification_codes(&client)?;
    prompt_user_to_copy_code(&codes.user_code)?;
    open_browser(&codes.verification_uri);
    let token = poll_for_token(&client, codes.device_code, codes.interval)?;

    let entry = keyring::Entry::new(SERVICE, USERNAME)?;
    entry.set_password(&token)?;

    Ok(())
}

fn request_device_and_user_verification_codes(
    client: &reqwest::blocking::Client,
) -> Result<GitHubDeviceCodesResponse> {
    let open_url = url::Url::parse(GITHUB_DEVICE_LOGIN_CODE_URL)?;
    let body = GitHubDeviceCodesRequest {
        client_id: GITHUB_APP_CLIENT_ID.to_string(),
        scope: "repo".to_string(),
    };

    let res = client
        .post(open_url)
        .json(&body)
        .header("Accept", "application/json")
        .send()?
        .error_for_status()?
        .json()?;

    Ok(res)
}

fn prompt_user_to_copy_code(code: &str) -> Result<()> {
    println!("Attempting to authenticate with GitHub...");
    println!("  1. Copy your one time code: {code}");
    print!("  2. Press ENTER to open up web browser to paste code...");
    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}

fn open_browser(url: &str) {
    match open::that(url) {
        Ok(_status) => {
            println!("Opened GitHub Login in web browser");
        }
        Err(e) => {
            println!("Error opening web browser: {e}");
            println!("Please click here to manually open web browser: {url}");
        }
    }
}

fn poll_for_token(
    client: &reqwest::blocking::Client,
    device_code: String,
    interval: u32,
) -> Result<String> {
    let check_req = GitHubAccesCheckRequest {
        client_id: GITHUB_APP_CLIENT_ID.to_string(),
        device_code,
        grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
    };

    let req = client
        .post(GITHUB_DEVICE_LOGIN_CHECK_URL)
        .json(&check_req)
        .header("Accept", "application/json");

    let token = loop {
        let res: serde_json::Value = req
            .try_clone()
            .unwrap()
            .send()?
            .error_for_status()?
            .json()?;

        if let Some(error) = res["error"].as_str() {
            let error: GitHubDeviceFlowErrorCode = GitHubDeviceFlowErrorCode::from_str(error)?;
            match error {
                GitHubDeviceFlowErrorCode::AccessDenied => {
                    anyhow::bail!("User clicked cancel. Unable to sign into GitHub")
                }
                GitHubDeviceFlowErrorCode::AuthorizationPending => {
                    std::thread::sleep(std::time::Duration::from_secs(interval as u64));
                }
                GitHubDeviceFlowErrorCode::SlowDown => {
                    std::thread::sleep(std::time::Duration::from_secs(interval as u64 + 5));
                }
                GitHubDeviceFlowErrorCode::ExpiredToken => {
                    anyhow::bail!("Device code expired. Please re-run login")
                }
                GitHubDeviceFlowErrorCode::UnsupportedGrantType => {
                    anyhow::bail!("Unsupported grant type. This is against the spec?")
                }
                GitHubDeviceFlowErrorCode::IncorrectClientCredentials => {
                    anyhow::bail!("Incorrect client credentials.")
                }
                GitHubDeviceFlowErrorCode::IncorrectDeviceCode => {
                    anyhow::bail!("Incorrect device code.")
                }
            }
        } else {
            let access_token_response: GitHubAccesCheckResponse = serde_json::from_value(res)?;
            println!("GitHub Login Successful");
            break access_token_response.access_token;
        }
    };

    Ok(token)
}
