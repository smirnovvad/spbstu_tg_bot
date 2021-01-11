extern crate chrono;
extern crate reqwest;
extern crate serde_json;

use self::reqwest::Url;
use chrono::prelude::*;
use chrono::Duration;
use serde_json::Value;

async fn parse_group(link: &str, days: i64) -> Option<Value> {
    let dt: DateTime<Local> = Local::now() + Duration::days(days);
    let api = "http://ruz2.spbstu.ru/api/v1/ruz/scheduler/";
    let uri = match Url::parse(&format!(
        "{}{}?date={}",
        api,
        link,
        dt.format("%Y-%m-%d").to_string()
    )) {
        Ok(res) => res,
        Err(err) => panic!("{:?}", err),
    };
    let mut body = match reqwest::blocking::get(uri) {
        Ok(res) => res,
        Err(err) => panic!("{:?}", err),
    };
    let body = match body.text() {
        Ok(res) => res,
        Err(err) => panic!("{:?}", err),
    };
    // println!("{:?}", body);
    let week: Value = serde_json::from_str(&*body).expect("JSON was not well-formatted");

    Some(week)
}

pub async fn parse_week(link: &str, days: i64) -> String {
    let mut out = String::new();
    let week: Value = parse_group(link, days).await.unwrap();
    let week = week["week"].as_object().unwrap();
    let date_start = &week["date_start"].as_str().unwrap_or("");
    let date_end = &week["date_end"].as_str().unwrap_or("");
    let is_odd = &week["is_odd"].as_bool().unwrap_or(true);
    out += &format!(
        "*{} - {}* {}",
        date_start,
        date_end,
        if *is_odd {
            "Нечетная *Ⅰ*"
        } else {
            "Четная *Ⅱ*"
        }
    );
    out
}

pub async fn parse_day(link: &str, date: u32, days: i64) -> String {
    let mut out = String::new();
    let week: Value = parse_group(link, days).await.unwrap();
    let days = week["days"].as_array().unwrap();
    // println!("{:?}", days);
    let date = date % 7;
    if !days.is_empty() {
        for day in days {
            if day["weekday"] == date {
                // println!("{:?}", day);
                // let tmp = Vec::<Value>::new();
                let lessons = day["lessons"].as_array().unwrap();
                // println!("{:?}", lessons.len());
                if !lessons.is_empty() {
                    for lesson in day["lessons"].as_array().unwrap() {
                        let subj = &lesson["subject"].as_str().unwrap_or("");
                        let subj_type = &lesson["typeObj"]["name"].as_str().unwrap_or("");
                        let time_start = &lesson["time_start"].as_str().unwrap_or("");
                        let time_end = &lesson["time_end"].as_str().unwrap_or("");
                        let building = &lesson["auditories"][0]["building"]["name"]
                            .as_str()
                            .unwrap_or("");
                        let aud = &lesson["auditories"][0]["name"].as_str().unwrap_or("");
                        out += &format!(
                            "*<{} - {}>* - {} ({}) 📍 {}, ауд. {}",
                            time_start, time_end, subj, subj_type, building, aud
                        );
                        out += "\n";
                    }
                } else {
                    out = "Тут ничего нет :С".to_string();
                }
            }
        }
    } else {
        out = "Тут ничего нет :С".to_string();
    }
    if date == 7 {
        out = "Тут ничего нет :С".to_string();
    }
    out
}
