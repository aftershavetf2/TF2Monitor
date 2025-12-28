use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "comments")]
pub struct Model {
    /// Auto-increment primary key
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,

    /// SteamID64 of account (Foreign Key to Account)
    pub steam_id: i64,

    /// SteamID64 of writer of comment (Foreign Key to Account)
    pub writer_steam_id: i64,

    /// Name of writer of the comment (max 32 visible characters, UTF-8)
    pub writer_name: String,

    /// Comment text
    pub comment: String,

    /// UnixTime when comment was first seen
    pub created_date: i64,

    /// UnixTime when the comment no longer was found on the account (nullable)
    #[sea_orm(nullable)]
    pub deleted_date: Option<i64>,
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
        from = "Column::WriterSteamId",
        to = "super::account::Column::SteamId"
    )]
    WriterAccount,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

