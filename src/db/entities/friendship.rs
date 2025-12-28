use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "friendship")]
pub struct Model {
    /// SteamID64 of account (Composite Primary Key, Foreign Key to Account)
    #[sea_orm(primary_key)]
    pub steam_id: i64,

    /// SteamID64 of friend (Composite Primary Key, Foreign Key to Account)
    #[sea_orm(primary_key)]
    pub friend_steam_id: i64,

    /// Name of friend (max 32 visible characters, UTF-8)
    pub friend_name: String,

    /// UnixTime when they first was found to be friends
    pub friend_date: i64,

    /// UnixTime when they no longer were found to be friends (nullable)
    #[sea_orm(nullable)]
    pub unfriend_date: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::SteamId",
        to = "super::account::Column::SteamId"
    )]
    Account,

    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::FriendSteamId",
        to = "super::account::Column::SteamId"
    )]
    FriendAccount,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

