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
    let url = format!(
        "https://steamcommunity.com/comment/Profile/render/{}/-1/?start=0&totalcount=338&count=50&sessionid=&feature2=-1",
        steam_id
    );

    let data = reqwest::blocking::get(&url)?.text()?;
    Ok(serde_json::from_str::<Reply>(&data)?)
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
        let avatar_thumb = comment
            .select(&scraper::Selector::parse(".commentthread_comment_avatar img").unwrap())
            .next()
            .and_then(|image| image.value().attr("src"))
            .map(|url| {
                if url.starts_with("//") {
                    format!("https:{url}")
                } else {
                    url.to_owned()
                }
            })
            .unwrap_or_default();
        let body = comment
            .select(&scraper::Selector::parse("div.commentthread_comment_text").unwrap())
            .next()
            .unwrap();
        let comment = comment_text(body);

        comments.push(SteamProfileComment {
            name: author,
            avatar_thumb,
            steamid: SteamID::from_steam_id32(&author_steamid32),
            comment,
        });
    }

    comments
}

/// Keep Steam's explicit line breaks and the spacing used by Unicode drawings.
fn comment_text(body: scraper::ElementRef<'_>) -> String {
    use scraper::node::Node;

    let mut nodes = body
        .descendants()
        .filter(|node| match node.value() {
            Node::Text(_) => true,
            Node::Element(element) => element.name() == "br",
            _ => false,
        })
        .peekable();
    let mut text = String::new();
    let mut first = true;
    while let Some(node) = nodes.next() {
        match node.value() {
            Node::Text(value) => {
                let mut value: &str = value;
                // Steam pads the body with source newlines and tabs. Remove only
                // that padding, preserving spaces, NBSPs and explicit <br> lines.
                if first {
                    value = value.trim_start_matches(['\r', '\n', '\t']);
                }
                if nodes.peek().is_none() {
                    value = value.trim_end_matches(['\r', '\n', '\t']);
                }
                text.push_str(value);
            }
            Node::Element(_) => text.push('\n'),
            _ => unreachable!(),
        }
        first = false;
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_body(body: &str) -> String {
        let html = format!(
            r#"<div class="commentthread_comment">
                <a class="commentthread_author_link" data-miniprofile="123">Artist</a>
                <div class="commentthread_comment_text">{body}</div>
            </div>"#
        );
        parse_comments(&html).remove(0).comment
    }

    #[test]
    fn preserves_unicode_art_line_breaks_and_spacing() {
        assert_eq!(
            parse_body("\n\t\t  ░█  <br> ╔══╗<br/> ╚══╝  \t\t"),
            "  ░█  \n ╔══╗\n ╚══╝  "
        );
    }

    #[test]
    fn preserves_blank_lines_and_decodes_nested_text_and_entities() {
        assert_eq!(
            parse_body("\n\t<br><br>A <b>&amp;</b> B&nbsp; C<br><br>\t\n"),
            "\n\nA & B\u{a0} C\n\n"
        );
        assert_eq!(parse_body("  spaced  "), "  spaced  ");
        assert_eq!(parse_body(""), "");
    }

    #[test]
    fn parses_comment_author_thumbnail_and_handles_missing_images() {
        let comments = parse_comments(
            r#"
            <div class="commentthread_comment">
                <div class="commentthread_comment_avatar"><a><img src="//avatars.steamstatic.com/example.jpg"></a></div>
                <a class="commentthread_author_link" data-miniprofile="123">Example Player</a>
                <div class="commentthread_comment_text">Hello &amp; welcome!</div>
            </div>
            <div class="commentthread_comment">
                <a class="commentthread_author_link" data-miniprofile="456">Another Player</a>
                <div class="commentthread_comment_text">No avatar</div>
            </div>
        "#,
        );
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].name, "Example Player");
        assert_eq!(
            comments[0].avatar_thumb,
            "https://avatars.steamstatic.com/example.jpg"
        );
        assert_eq!(comments[0].comment, "Hello & welcome!");
        assert!(comments[1].avatar_thumb.is_empty());
        assert_eq!(comments[1].comment, "No avatar");
    }
}
