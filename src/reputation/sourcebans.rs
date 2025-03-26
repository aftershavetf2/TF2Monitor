use crate::models::steamid::SteamID;

#[derive(Debug, Clone)]
pub enum SourceBanParser {
    // Data is stored in a <ul> element
    Ul,
    // Data is stored in a <table> element
    Table,
}

#[derive(Debug, Clone)]
pub struct SourceBanSource {
    pub name: String,
    pub url: String,
    pub parser: SourceBanParser,
}

impl SourceBanSource {
    pub fn new(name: &str, url: &str, parser: SourceBanParser) -> SourceBanSource {
        SourceBanSource {
            name: name.to_string(),
            url: url.to_string(),
            parser,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceBan {
    pub source: String,
    pub steamid: SteamID,
    pub when: String,
    pub ban_length: String,
    pub reason: String,
}

// Test subject:
// - Multiple bans: https://steamhistory.net/id/76561198398458549
// - Multiple bans: https://steamhistory.net/id/76561199163606348
//
fn get_sources() -> Vec<SourceBanSource> {
    // TODO: Add more sources from https://steamhistory.net/sources
    vec![
        SourceBanSource::new(
            "UGC-Gaming.net",
            "https://sb.ugc-gaming.net/index.php?p=banlist&advSearch={}&advType=steamid",
            SourceBanParser::Ul,
        ),
        SourceBanSource::new(
            "blackwonder.tf",
            "https://bans.blackwonder.tf/index.php?p=banlist&advSearch={}&advType=steamid",
            SourceBanParser::Ul,
        ),
        SourceBanSource::new(
            "flux.tf",
            "https://bans.flux.tf/index.php?p=banlist&advSearch={}&advType=steamid",
            SourceBanParser::Table,
        ),
        SourceBanSource::new(
            "dpg.tf",
            "https://bans.dpg.tf/index.php?p=banlist&advSearch={}&advType=steamid",
            SourceBanParser::Table,
        ),
        SourceBanSource::new(
            "skial.com",
            "https://www.skial.com/sourcebans/index.php?p=banlist&advSearch={}&advType=steamid",
            SourceBanParser::Table,
        ),
        //
        // The following source bans are not working because they are behind CloudFlare.
        // SourceBanSource::new(
        //     "panda-community.com",
        //     "https://bans.panda-community.com/index.php?p=banlist&advSearch={}&advType=steamid",
        //     SourceBanParser::Ul,
        // ),
        //
    ]
}

pub fn get_source_bans(steamid: SteamID) -> Vec<SourceBan> {
    let sources = get_sources();
    let mut result = vec![];
    for source in sources {
        if let Some(bans) = get_source_ban(&source, steamid) {
            for ban in bans {
                result.push(ban);
            }
        }
    }

    result
}

fn get_source_ban(source: &SourceBanSource, steamid: SteamID) -> Option<Vec<SourceBan>> {
    let url = source.url.replace("{}", &steamid.to_steam_id());

    log::info!("SourceBans: Getting bans from {}", url);

    let html = get_html(&url)?;

    let document = scraper::Html::parse_document(html.as_str());

    match source.parser {
        SourceBanParser::Ul => parse_source_ban_ul(&html, source, &document),
        SourceBanParser::Table => parse_source_ban_table(source, &document),
    }
}

fn get_html(url: &str) -> Option<String> {
    match reqwest::blocking::get(url) {
        Ok(resp) => match resp.text() {
            Ok(text) => Some(text),
            Err(err) => {
                log::error!("SourceBans: Failed to get text from response: {err}");
                None
            }
        },
        Err(err) => {
            log::error!("SourceBans: Failed to get text from response: {err}");
            None
        }
    }
}

fn parse_source_ban_ul(
    html: &str,
    source: &SourceBanSource,
    document: &scraper::Html,
) -> Option<Vec<SourceBan>> {
    log::info!("Parsing {} using ul parser", source.name);

    if !html.contains("ban_list_detal") {
        return None;
    }

    let selector = scraper::Selector::parse("ul.ban_list_detal").unwrap();

    let mut result: Vec<SourceBan> = vec![];

    for element in document.select(&selector) {
        if let Some(ban) = parse_source_ban_one_ul(source, &element) {
            result.push(ban);
        }
    }

    Some(result)
}

fn parse_source_ban_one_ul(
    source: &SourceBanSource,
    element: &scraper::ElementRef,
) -> Option<SourceBan> {
    let selector_li = scraper::Selector::parse("li").unwrap();

    let mut steamid: Option<SteamID> = None;
    let mut when: Option<String> = None;
    let mut ban_length: Option<String> = None;
    let mut reason: Option<String> = None;

    // Go through all the li elements
    for el2 in element.select(&selector_li) {
        /*
          <li>
            <span><i class="fas fa-user"></i> Player</span>
            <span>Lubbeek</span>
          </li>
        */
        let texts = el2
            .text()
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>();

        if texts.len() < 2 {
            continue;
        }

        let label = *texts.first()?;
        let value = *texts.get(1)?;

        match label {
            "Steam3 ID" => {
                steamid = Some(SteamID::from_steam_id32(value));
            }
            "Invoked on" => {
                when = Some(value.to_string());
            }
            "Ban length" => {
                ban_length = Some(value.to_string());
            }
            "Reason" => {
                reason = Some(value.to_string());
            }
            _ => {}
        }
    }

    let steamid = steamid?;
    let when = when?;
    let ban_length = ban_length?;
    let reason = reason?;
    let source = source.name.clone();

    let ban = SourceBan {
        source,
        steamid,
        when,
        ban_length,
        reason,
    };

    Some(ban)
}

fn parse_source_ban_table(
    source: &SourceBanSource,
    document: &scraper::Html,
) -> Option<Vec<SourceBan>> {
    log::info!("Parsing {} using table parser", source.name);

    let selector_div = scraper::Selector::parse("div#banlist").unwrap();
    let selector_table = scraper::Selector::parse("table.listtable").unwrap();

    let div = document.select(&selector_div).next()?;

    let table = div.select(&selector_table).next()?;

    let mut result: Vec<SourceBan> = vec![];

    for element in table.select(&selector_table) {
        if let Some(ban) = parse_source_ban_one_table(source, &element) {
            result.push(ban);
        }
    }

    Some(result)
}

fn parse_source_ban_one_table(
    source: &SourceBanSource,
    element: &scraper::ElementRef,
) -> Option<SourceBan> {
    let selector_tr = scraper::Selector::parse("tr").unwrap();
    let selector_td = scraper::Selector::parse("td").unwrap();
    let selector_a = scraper::Selector::parse("a").unwrap();

    let mut steamid: Option<SteamID> = None;
    let mut when: Option<String> = None;
    let mut ban_length: Option<String> = None;
    let mut reason: Option<String> = None;

    // Go through all the tr elements
    for row_el in element.select(&selector_tr) {
        /*
        <tr align="left">
            <td width="20%" height="16" class="listtable_1">Steam3 ID</td>
            <td height="16" class="listtable_1">
                <a href="http://steamcommunity.com/profiles/[U:1:438192821]" target="_blank">[U:1:438192821]</a>
            </td>
        </tr>
        */

        // Extract the texts from the td elements, first td has the label and the second td has the value
        let label = row_el.select(&selector_td).next()?.text().next()?.trim();

        match label {
            "Steam3 ID" => {
                // Extract text from the second td element inside the a element
                let value = row_el
                    .select(&selector_td)
                    .nth(1)?
                    .select(&selector_a)
                    .next()?
                    .text()
                    .next()?
                    .trim();
                steamid = Some(SteamID::from_steam_id32(value));
            }
            "Invoked on" => {
                let value = row_el.select(&selector_td).nth(1)?.text().next()?.trim();
                when = Some(value.to_string());
            }
            "Banlength" => {
                let value = row_el.select(&selector_td).nth(1)?.text().next()?.trim();
                ban_length = Some(value.to_string());
            }
            "Reason" => {
                let value = row_el.select(&selector_td).nth(1)?.text().next()?.trim();
                reason = Some(value.to_string());
            }
            _ => {}
        }
    }

    let steamid = steamid?;
    let when = when?;
    let ban_length = ban_length?;
    let reason = reason?;
    let source = source.name.clone();

    let ban = SourceBan {
        source,
        steamid,
        when,
        ban_length,
        reason,
    };

    Some(ban)
}
