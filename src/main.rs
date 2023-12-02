#![windows_subsystem = "windows"]

use std::error::Error;

use chrono::{DateTime, Duration};

use chrono::prelude::*;
use dioxus::prelude::*;
use serde::Deserialize;

const OBERSCHLEISSHEIM_URL: &str = "https://www.mvg.de/api/fib/v2/departure?globalId=de:09184:2000&limit=10&offsetInMinutes=0&transportTypes=SBAHN";

enum TransportType {
    Sbahn,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDeparture {
    #[serde(rename = "plannedDepartureTime")]
    planned_departure_time_ms: u64,
    #[serde(rename = "realtime")]
    real_time: bool,
    #[serde(rename = "delayInMinutes", default)]
    delay_minutes: u16,
    #[serde(rename = "realtimeDepartureTime")]
    real_departure_time_ms: u64,
    transport_type: String,
    #[serde(rename = "label")]
    vehicle_label: String,
    diva_id: String,
    network: String,
    train_type: String,
    destination: String,
    cancelled: bool,
    sev: bool,
    platform: u16,
    messages: Vec<String>,
    banner_hash: String,
    occupancy: String,
    stop_point_global_id: String,
}

#[derive(PartialEq)]
struct Departure {
    planned: DateTime<Local>,
    delay: Duration,
    destination: String,
    cancelled: bool,
}

impl From<RawDeparture> for Departure {
    fn from(value: RawDeparture) -> Self {
        let planned = Local
            .timestamp_millis_opt(value.planned_departure_time_ms as i64)
            .unwrap();
        let delay = Duration::minutes(value.delay_minutes as i64);
        Departure {
            planned,
            delay,
            destination: value.destination,
            cancelled: value.cancelled,
        }
    }
}

#[inline_props]
fn ResponseTile<'a>(cx: Scope, departure: &'a Departure) -> Element {
    let planned = departure.planned.format("%H:%M");
    let delay = departure.delay.num_minutes();
    let inner = rsx!("[{departure.destination}] {planned}: +{delay}");
    cx.render(rsx!(
        div {
            if departure.cancelled {
                rsx!(s { inner})
            } else {
                rsx!(inner)
            }
        }
    ))
}

async fn get_response() -> Result<Vec<Departure>, Box<dyn Error>> {
    Ok(reqwest::get(OBERSCHLEISSHEIM_URL)
        .await?
        .json::<Vec<RawDeparture>>()
        .await?
        .into_iter()
        .map(Departure::from)
        .collect::<Vec<_>>())
}

fn app(cx: Scope) -> Element {
    let current_response = use_state(cx, || None);
    let is_fetching = use_state(cx, || false);
    let _: &Coroutine<()> = use_coroutine(cx, |_rx| {
        let is_fetching = is_fetching.to_owned();
        let current_response = current_response.to_owned();
        async move {
            loop {
                is_fetching.set(true);
                current_response.set(Some(get_response().await));
                is_fetching.set(false);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    });
    let time = use_state(cx, Local::now);
    let _: &Coroutine<()> = use_coroutine(cx, |_rx| {
        let time = time.to_owned();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                time.set(Local::now());
            }
        }
    });

    let time = Local::now().format("%H:%M:%S");
    let tile_body = match current_response.get() {
        Some(Ok(responses)) => {
            rsx! {
                responses.iter().map(|response| {
                    rsx!(ResponseTile {
                        departure: response
                    })
                })
            }
        }
        Some(Err(e)) => rsx! { "Fetching data failed: {e}"  },
        None => rsx! { ""  },
    };
    cx.render(rsx!(div {class: "parent", div {class: "child", "{time}"}, if *is_fetching.get() { rsx!(div {class: "child", div {class: "loader"}}) }  }, div {tile_body }))
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();
    hot_reload_init!();
    dioxus_desktop::launch_cfg(
        app,
        dioxus_desktop::Config::new()
            .with_custom_head(r#"<link rel="stylesheet" href="public/tailwind.css">"#.to_string()),
    )
}
