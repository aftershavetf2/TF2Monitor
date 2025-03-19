use super::SteamProfileComment;
use crate::models::steamid::SteamID;
use serde::Deserialize;
use std::error::Error;

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
    // https://steamcommunity.com/profiles/76561197974228301/allcomments
    // let url = format!(
    //     "https://steamcommunity.com/profiles/{}/allcomments",
    //     steam_id
    // );
    let url = format!(
        "https://steamcommunity.com/comment/Profile/render/{}/-1/?start=0&totalcount=338&count=50&sessionid=&feature2=-1",
        steam_id
    );

    let resp: Reply = reqwest::blocking::get(&url)?.json()?;
    // println!("{}", resp.comments_html);

    Ok(resp)
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
