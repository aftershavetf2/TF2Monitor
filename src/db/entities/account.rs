use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "account")]
pub struct Model {
    /// SteamID64 of account (Primary Key)
    #[sea_orm(primary_key)]
    pub steam_id: i64,

    /// Account name (max 32 visible characters, UTF-8)
    pub name: String,

    /// UnixTime when account was created, approximated if private (nullable)
    #[sea_orm(nullable)]
    pub created_date: Option<i64>,

    /// Number of minutes playing TF2 (nullable)
    #[sea_orm(nullable)]
    pub tf2_time: Option<i64>,

    /// URL to avatar image
    pub avatar_url: String,

    /// Whether profile is public
    pub public_profile: bool,

    /// UnixTime when account data was last updated
    pub last_updated: i64,

    /// UnixTime when friend list was last fetched (nullable)
    #[sea_orm(nullable)]
    pub friends_fetched: Option<i64>,

    /// UnixTime when comments was last fetched (nullable)
    #[sea_orm(nullable)]
    pub comments_fetched: Option<i64>,

    /// UnixTime when account data was last fetched
    pub fetch_date: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::friendship::Entity")]
    Friendships,

    #[sea_orm(has_many = "super::comments::Entity")]
    Comments,
}

impl ActiveModelBehavior for ActiveModel {}
