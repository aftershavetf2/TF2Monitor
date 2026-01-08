# Data model

## General Rules

- SQLite is the database engine used for this project.
- SeaORM is the database abstraction framework for this project.
- SteamID64 is stored as a 64-bit unsigned integer in the database. For SQLite this means an INTEGER.
- Names of accounts on Steam Community has a max length of 32 visible characters.
  - Due to UTF8 encoding, 32 emojis could use several bytes per char.
  - In the database the datatype TEXT will be used.

## SteamWebAPI

This is the data model for data fetched from SteamWebAPI.

**Status**: Implemented. Data is stored in SQLite and cached with TTL timestamps.

**Primary Keys**:

- `Account.steam_id` (PRIMARY KEY)
- `Friendship`: Composite primary key (`steam_id`, `friend_steam_id`)
- `Comments`: Auto-increment `id` (PRIMARY KEY)

**Foreign Keys**:

- `Friendship.steam_id` → `Account.steam_id`
- `Friendship.friend_steam_id` → `Account.steam_id`
- `Comments.steam_id` → `Account.steam_id`
- `Comments.writer_steam_id` → `Account.steam_id`

**Indexes**:

- `Account.steam_id` (PRIMARY KEY, automatically indexed)
- `Friendship.steam_id` (INDEX for lookups by account)
- `Friendship.friend_steam_id` (INDEX for reverse lookups)
- `Comments.steam_id` (INDEX for lookups by account)
- `Comments.writer_steam_id` (INDEX for lookups by comment writer)
- `Comments.created_date` (INDEX for time-based queries)

**Constraints**:

- `Account.name`: TEXT, max 32 visible characters (UTF-8, variable byte length)
- `Friendship.friend_name`: TEXT, max 32 visible characters
- `Comments.writer_name`: TEXT, max 32 visible characters
- `Comments.comment`: TEXT, max length TBD (Steam API limit)

**Nullable Fields**:

- `Account.created_date`: NULL if profile is private and approximation failed
- `Account.tf2_time`: NULL if not available
- `Account.friends_fetched`: NULL if never fetched
- `Account.comments_fetched`: NULL if never fetched
- `Friendship.unfriend_date`: NULL if still friends
- `Comments.deleted_date`: NULL if comment still exists

```mermaid
erDiagram
    Account {
        integer steam_id PK "SteamID64 of account (Primary Key)"
        string name "Max 32 visible characters (UTF-8)"
        integer created_date "UnixTime when account was created, approximated if private (nullable)"
        integer tf2_time "Number of minutes playing TF2 (nullable)"
        string avatar_url "URL to avatar image"
        boolean public_profile "Whether profile is public"
        integer last_updated "UnixTime when account data was last updated"
        integer friends_fetched "UnixTime when friend list was last fetched (nullable)"
        integer comments_fetched "UnixTime when comments was last fetched (nullable)"
        integer fetch_date "UnixTime when account data was last fetched"
    }

    Friendship {
        integer steam_id PK,FK "SteamID64 of account (Composite Primary Key, Foreign Key to Account)"
        integer friend_steam_id PK,FK "SteamID64 of friend (Composite Primary Key, Foreign Key to Account)"
        string  friend_name "Name of friend (Max 32 visible characters)"
        integer friend_date "UnixTime when they first was found to be friends"
        integer unfriend_date "UnixTime when they no longer were found to be friends (nullable)"
    }

    Comments {
        integer id PK "Auto-increment primary key"
        integer steam_id FK "SteamID64 of account (Foreign Key to Account)"
        integer writer_steam_id FK "SteamID64 of writer of comment (Foreign Key to Account)"
        string  writer_name "Name of writer of the comment (Max 32 visible characters)"
        string  comment "Comment text (Max length TBD)"
        integer created_date "UnixTime when comment was first seen"
        integer deleted_date "UnixTime when the comment no longer was found on the account (nullable)"
    }

    Account ||--o{ Comments : "one account can have zero to many comments"
    Account ||--o{ Friendship : "one account can have zero to many friends"
    Account ||--o{ Friendship : "one account can be friend to zero to many accounts (reverse)"
    Account ||--o{ Comments : "one account can write zero to many comments (as writer)"

```

## Cache Strategy

The database serves as the primary cache for Steam API data. Cache freshness is managed via TTL timestamps:

- **Account data**: Cached in `account` table, refreshed based on `last_updated`
- **Friendship data**: Cached in `friendship` table, refreshed based on `friends_fetched` timestamp
- **Comments data**: Cached in `comments` table, refreshed based on `comments_fetched` timestamp
- **Playtime data**: Cached in `playtime` table, refreshed based on `playtimes_fetched` timestamp
- **Steam bans**: Cached in `bans` table, refreshed based on `steam_bans_last_fetched` timestamp

TTL values are configured in `src/config.rs`.
