use std::path::PathBuf;

use anyhow::{anyhow, Result};
use chrono::{prelude::*, DateTime, Duration, Local};
use itertools::Itertools;
use log::{debug, info};
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct Record {
    #[serde(deserialize_with = "deserialize_datetime")]
    pub unix_begin: DateTime<Local>,
    #[serde(deserialize_with = "deserialize_datetime")]
    pub unix_end: DateTime<Local>,
    pub duration_decimal: f32,
    pub folder: String,
    pub task: String,
    pub hourly_rate: Option<f32>,
    pub billing_status: Option<BillingStatus>,
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp: i64 = Deserialize::deserialize(deserializer)?;
    let utc_timestamp =
        DateTime::<Utc>::from_timestamp(timestamp, 0).expect("Failed to parse timestamp");
    let local_timestamp = utc_timestamp.with_timezone(&Local);
    Ok(local_timestamp)
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BillingStatus {
    Billed,
    Unbilled,
    Paid,
}

pub fn parse_records(path: &PathBuf) -> Result<Vec<Record>> {
    debug!("Opening {:?}", path);
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b';')
        .from_path(path)?;
    let records: Vec<Record> = rdr.deserialize().collect::<Result<_, _>>()?;
    debug!("Read {} records", records.len());
    let records = records
        .into_iter()
        .filter(|r| r.hourly_rate.is_some())
        .collect::<Vec<_>>();
    info!("Read {} records with hourly rate", records.len());

    Ok(records)
}

pub fn handle(path: &PathBuf) -> Result<()> {
    let records = parse_records(path)?;

    let first_day = records
        .iter()
        .min_by_key(|r| r.unix_begin)
        .ok_or_else(|| anyhow!("No records"))?
        .unix_begin;
    let first_day = first_day.date_naive();
    let last_day = records
        .iter()
        .max_by_key(|r| r.unix_end)
        .ok_or_else(|| anyhow!("No records"))?
        .unix_end;
    let last_day = last_day.date_naive();

    debug!("First day: {:?}", first_day);
    debug!("Last day: {:?}", last_day);

    for offset_days in 0..=(last_day - first_day).num_days() {
        let day = first_day + Duration::days(offset_days);

        let all_today = records
            .iter()
            .filter(|r| r.unix_begin.date_naive() == day && r.unix_end.date_naive() == day)
            .collect::<Vec<_>>();
        let total_today = all_today.iter().map(|r| r.duration_decimal).sum::<f32>();

        debug!("Day: {:?}; total: {}h", day, total_today);
    }

    Ok(())
}

const MIN_EXTENSION_MINUTES: i64 = 10;

fn split_record(record: Record) -> Vec<Record> {
    if record.unix_begin.date_naive() == record.unix_end.date_naive() {
        return vec![record.clone()];
    }

    let mut left = record.clone();
    left.unix_end = Local
        .with_ymd_and_hms(
            left.unix_begin.year(),
            left.unix_begin.month(),
            left.unix_begin.day(),
            23,
            59,
            00,
        )
        .single()
        .expect("Failed to create DateTime");

    let mut right = record.clone();
    right.unix_begin = Local
        .with_ymd_and_hms(
            right.unix_end.year(),
            right.unix_end.month(),
            right.unix_end.day(),
            00,
            00,
            00,
        )
        .single()
        .expect("Failed to create DateTime");

    let left_split = split_record(left);
    let right_split = split_record(right);

    left_split
        .into_iter()
        .chain(right_split.into_iter())
        .collect_vec()
}

#[derive(Debug, Serialize, Clone)]
struct LoginArgs {
    username: String,
    password: String,
    login: bool,
}

pub async fn upload(records: Vec<Record>, username: &str, password: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let auth_args = LoginArgs {
        username: username.to_string(),
        password: password.to_string(),
        login: true,
    };
    let res = client
        .post("https://hiwi.embedded.rwth-aachen.de/?json")
        .form(&auth_args)
        .send()
        .await?;
    let cookie = res
        .headers()
        .get("set-cookie")
        .ok_or_else(|| anyhow!("Failed to get cookie from response headers"))?;
    let cookie = cookie
        .to_str()?
        .split_once(';')
        .expect("Set-Cookie should contain at least one ';'")
        .0;

    debug!("Obtained auth cookie: {}", cookie);

    let to_upload = records
        .iter()
        .filter(|r| r.billing_status == Some(BillingStatus::Unbilled))
        .collect::<Vec<_>>();
    info!("Uploading {} unbilled records", to_upload.len());

    for record in to_upload {
        debug!("Uploading record: {:?}", record);

        // TODO: Take dynamic segment from entry
        let note = format!(
            "{}: {}\n\nUploaded by: {} v{} [{} UTC]",
            record.folder,
            record.task,
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        );

        let end = match record.unix_end.format("%H:%M").to_string().as_str() {
            "00:00" => "24:00".to_string(),
            end => end.to_string(),
        };

        let args = [
            ("hw_begin", record.unix_begin.format("%H:%M").to_string()),
            ("hw_end", end),
            ("hw_breaktime", "".to_string()), // TODO: Auto-generate this
            ("hw_note", note),
            ("hw_date", record.unix_begin.format("%Y-%m-%d").to_string()),
            ("savetimes", "Eintragen".to_string()),
        ];

        let _ = client
            .post("https://hiwi.embedded.rwth-aachen.de/?json")
            .header("Cookie", cookie)
            .form(&args)
            .send()
            .await?;
    }

    Ok(())
}
