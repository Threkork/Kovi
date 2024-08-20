pub mod add;
pub mod new_kovi;
pub mod new_plugin;

static DEFAULT_PLUGIN_CODE: &str = r#"use kovi::PluginBuilder;

#[kovi::plugin]
pub fn main(mut plugin: PluginBuilder) {
    plugin.on_msg(move |event| {
        if event.text == Option::Some("hi".to_string()) {
            event.reply("hi")
        }
    });
}
"#;


static DEFAULT_MAIN_CODE: &str = r#"use kovi::build_bot;
fn main() {
    kovi::log::set_logger();
    build_bot!().run();
}
"#;

pub fn get_latest_version() -> Result<String, Box<dyn std::error::Error>> {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct CrateResponse {
        #[serde(rename = "crate")]
        crate_: CrateInfo,
    }

    #[derive(Deserialize, Debug)]
    struct CrateInfo {
        max_version: String,
    }

    let url = "https://crates.io/api/v1/crates/kovi".to_string();

    let client = reqwest::blocking::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("kovi cli (https://github.com/Threkork/Kovi)"),
    );

    let response = client.get(&url).headers(headers).send()?.text()?;
    let response: CrateResponse = serde_json::from_str(&response)?;
    Ok(response.crate_.max_version)
}
