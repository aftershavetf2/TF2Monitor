use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(50))")]
pub enum Game {
    #[sea_orm(string_value = "TF2")]
    Tf2,
    #[sea_orm(string_value = "CS2")]
    Cs2,
    #[sea_orm(string_value = "CSGO")]
    Csgo,
    #[sea_orm(string_value = "DOTA2")]
    Dota2,
    #[sea_orm(string_value = "PUBG")]
    Pubg,
    #[sea_orm(string_value = "APEX")]
    Apex,
    #[sea_orm(string_value = "RUST")]
    Rust,
    #[sea_orm(string_value = "GTA5")]
    Gta5,
    #[sea_orm(string_value = "OTHER")]
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "playtime")]
pub struct Model {
    /// SteamID64 of account (Composite Primary Key, Foreign Key to Account)
    #[sea_orm(primary_key)]
    pub steam_id: i64,

    /// Game identifier (Composite Primary Key)
    #[sea_orm(primary_key)]
    pub game: Game,

    /// Number of minutes playing the game
    pub play_minutes: i64,

    /// UnixTime when playtime was last updated
    pub last_updated: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::SteamId",
        to = "super::account::Column::SteamId"
    )]
    Account,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}
