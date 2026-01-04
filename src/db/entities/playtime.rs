use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use serde::{Deserialize, Serialize};

use crate::db::schema::playtime;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow,
)]
#[diesel(sql_type = Text)]
pub enum Game {
    Tf2,
    Cs2,
    Csgo,
    Dota2,
    Pubg,
    Apex,
    Rust,
    Gta5,
    Valorant,
    Other,
}

impl ToSql<Text, Sqlite> for Game {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let s = match self {
            Game::Tf2 => "TF2",
            Game::Cs2 => "CS2",
            Game::Csgo => "CSGO",
            Game::Dota2 => "DOTA2",
            Game::Pubg => "PUBG",
            Game::Apex => "APEX",
            Game::Rust => "RUST",
            Game::Gta5 => "GTA5",
            Game::Valorant => "VALORANT",
            Game::Other => "OTHER",
        };
        ToSql::<Text, Sqlite>::to_sql(s, out)
    }
}

impl FromSql<Text, Sqlite> for Game {
    fn from_sql(value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<Text, Sqlite>>::from_sql(value)?;
        match s.as_str() {
            "TF2" => Ok(Game::Tf2),
            "CS2" => Ok(Game::Cs2),
            "CSGO" => Ok(Game::Csgo),
            "DOTA2" => Ok(Game::Dota2),
            "PUBG" => Ok(Game::Pubg),
            "APEX" => Ok(Game::Apex),
            "RUST" => Ok(Game::Rust),
            "GTA5" => Ok(Game::Gta5),
            "VALORANT" => Ok(Game::Valorant),
            "OTHER" => Ok(Game::Other),
            _ => Ok(Game::Other),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = playtime)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Playtime {
    /// SteamID64 of account (Composite Primary Key, Foreign Key to Account)
    pub steam_id: i64,

    /// Game identifier (Composite Primary Key)
    pub game: Game,

    /// Number of minutes playing the game (None = Unknown playtime)
    pub play_minutes: Option<i64>,

    /// UnixTime when playtime was last updated
    pub last_updated: i64,
}

#[derive(Clone, Debug, Insertable, AsChangeset)]
#[diesel(table_name = playtime)]
pub struct NewPlaytime {
    pub steam_id: i64,
    pub game: Game,
    pub play_minutes: Option<i64>,
    pub last_updated: i64,
}
