use super::SteamProfileComment;
use crate::{http_cache::get_from_cache_or_fetch, models::steamid::SteamID};
use serde::Deserialize;
use std::error::Error;

// Days to keep the cache
const DAYS_TO_KEEP: i32 = 30;

#[derive(Debug, Deserialize)]
pub struct Reply {
    pub success: bool,
    pub comments_html: String,
    // pub timelastpost: i64,
}

pub fn get_steam_profile_comments(steam_id: u64) -> Option<Vec<SteamProfileComment>> {
    let data = get_data(steam_id);
    match data {
        Ok(reply) => {
            if !reply.success {
                return None;
            }

            let comments = parse_comments(&reply.comments_html);
            Some(comments)
        }
        Err(_) => None,
    }
}

fn get_data(steam_id: u64) -> Result<Reply, Box<dyn Error>> {
    let url = format!(
        "https://steamcommunity.com/comment/Profile/render/{}/-1/?start=0&totalcount=338&count=50&sessionid=&feature2=-1",
        steam_id
    );

    if let Some(data) = get_from_cache_or_fetch(
        "Steam Profile Comments",
        &steam_id.to_string(),
        DAYS_TO_KEEP,
        &url,
    ) {
        if let Ok(reply) = serde_json::from_str::<Reply>(&data) {
            return Ok(reply);
        }
    }
    // Failed to parse the response, return None
    return Err("Failed to parse the response".into());
}

fn parse_comments(html: &str) -> Vec<SteamProfileComment> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("div.commentthread_comment").unwrap();
    let mut comments = Vec::new();

    for comment in document.select(&selector) {
        // println!("Comment: {:?}", comment);
        let author = comment
            .select(&scraper::Selector::parse("a.commentthread_author_link").unwrap())
            .next()
            .unwrap()
            .text()
            .collect::<String>()
            .trim()
            .to_string();
        let author_steamid32 = comment
            .select(&scraper::Selector::parse("a.commentthread_author_link").unwrap())
            .next()
            .unwrap()
            .value()
            .attr("data-miniprofile")
            .unwrap()
            .trim()
            .to_string();
        let comment = comment
            .select(&scraper::Selector::parse("div.commentthread_comment_text").unwrap())
            .next()
            .unwrap()
            .text()
            .collect::<String>()
            .trim()
            .to_string();

        comments.push(SteamProfileComment {
            name: author,
            steamid: SteamID::from_steam_id32(&author_steamid32),
            comment,
        });
    }

    comments
}
