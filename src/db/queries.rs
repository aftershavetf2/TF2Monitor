use diesel::prelude::*;

use super::entities::{
    Account, Ban, BanSource, Comment, Friendship, Game, NewAccount, NewBan, NewBanSource,
    NewComment, NewFriendship, NewPlayerFlag, NewPlaytime, PlayerFlag,
};
use super::schema::{account, ban_sources, bans, comments, friendship, player_flags, playtime};

/// Get all friendships for a given steam_id.
///
/// This function handles both direct and reverse friendships:
/// - Direct: Where the account has the given steam_id as the first key in the composite PK
///   (i.e., where steam_id = given_steam_id). In these results, `friend_steam_id` is the friend.
/// - Reverse: Where other accounts have the given steam_id as a friend
///   (i.e., where friend_steam_id = given_steam_id). In these results, `steam_id` is the friend.
///
/// This is necessary because private accounts may not have their friend list exposed,
/// but we can still find friendships from other public accounts that list this account as a friend.
///
/// # Arguments
/// * `conn` - Database connection
/// * `steam_id` - The SteamID64 to get friendships for
/// * `active_only` -
///   If `true`, returns only active friendships (where unfriend_date is NULL).
///   If `false`, returns both active and inactive friendships.
///
/// To extract the friend's steam_id from the result:
/// - For direct friendships: use `friendship.friend_steam_id`
/// - For reverse friendships: use `friendship.steam_id`
pub fn get_friendships(
    conn: &mut SqliteConnection,
    steam_id: i64,
    active_only: bool,
) -> Result<Vec<Friendship>, diesel::result::Error> {
    use friendship::dsl;

    // Build query for direct friendships: where steam_id = given_steam_id
    let mut direct_query = friendship::table
        .filter(dsl::steam_id.eq(steam_id))
        .into_boxed();

    if active_only {
        direct_query = direct_query.filter(dsl::unfriend_date.is_null());
    }
    let direct_friendships = direct_query.load::<Friendship>(conn)?;

    // Build query for reverse friendships: where friend_steam_id = given_steam_id
    // These represent accounts that have the given steam_id as a friend
    let mut reverse_query = friendship::table
        .filter(dsl::friend_steam_id.eq(steam_id))
        .into_boxed();

    if active_only {
        reverse_query = reverse_query.filter(dsl::unfriend_date.is_null());
    }
    let reverse_friendships = reverse_query.load::<Friendship>(conn)?;

    // Combine both lists
    // Note: We might get duplicates if a friendship exists in both directions,
    // but that's fine since the composite PK ensures uniqueness
    let mut all_friendships = direct_friendships;
    all_friendships.extend(reverse_friendships);

    Ok(all_friendships)
}

pub fn get_account_by_steam_id(
    conn: &mut SqliteConnection,
    steam_id: i64,
) -> Result<Option<Account>, diesel::result::Error> {
    use account::dsl;

    let result = account::table
        .filter(dsl::steam_id.eq(steam_id))
        .first::<Account>(conn)
        .optional()?;

    Ok(result)
}

/// Insert or update an account record.
/// When updating existing records, preserves timestamp fields (friends_fetched, comments_fetched,
/// playtimes_fetched, reputation_fetched) and only updates account info fields.
pub fn upsert_account(
    conn: &mut SqliteConnection,
    new_account: NewAccount,
) -> Result<(), diesel::result::Error> {
    use account::dsl;

    diesel::insert_into(account::table)
        .values(&new_account)
        .on_conflict(account::steam_id)
        .do_update()
        .set((
            dsl::name.eq(&new_account.name),
            dsl::created_date.eq(&new_account.created_date),
            dsl::avatar_thumb_url.eq(&new_account.avatar_thumb_url),
            dsl::avatar_full_url.eq(&new_account.avatar_full_url),
            dsl::public_profile.eq(&new_account.public_profile),
            dsl::last_updated.eq(&new_account.last_updated),
            // Note: We deliberately do NOT update the *_fetched timestamp fields here
            // to preserve them. They are managed by their respective update functions.
        ))
        .execute(conn)?;
    Ok(())
}

/// Insert or update a friendship record.
/// If the friendship already exists, updates the friend_name and keeps friend_date.
/// Sets unfriend_date to NULL (reactivates friendship if it was unfriended).
pub fn upsert_friendship(
    conn: &mut SqliteConnection,
    new_friendship: NewFriendship,
) -> Result<(), diesel::result::Error> {
    use friendship::dsl;

    diesel::insert_into(friendship::table)
        .values(&new_friendship)
        .on_conflict((dsl::steam_id, dsl::friend_steam_id))
        .do_update()
        .set((
            dsl::friend_name.eq(&new_friendship.friend_name),
            dsl::unfriend_date.eq::<Option<i64>>(None),
        ))
        .execute(conn)?;
    Ok(())
}

/// Mark a friendship as ended by setting unfriend_date.
pub fn mark_friendship_ended(
    conn: &mut SqliteConnection,
    steam_id: i64,
    friend_steam_id: i64,
    unfriend_date: i64,
) -> Result<(), diesel::result::Error> {
    use friendship::dsl;

    diesel::update(
        friendship::table.filter(
            dsl::steam_id
                .eq(steam_id)
                .and(dsl::friend_steam_id.eq(friend_steam_id)),
        ),
    )
    .set(dsl::unfriend_date.eq(Some(unfriend_date)))
    .execute(conn)?;
    Ok(())
}

/// Insert a new comment record.
/// Does not update if already exists (comments are immutable once created).
pub fn insert_comment(
    conn: &mut SqliteConnection,
    new_comment: NewComment,
) -> Result<(), diesel::result::Error> {
    diesel::insert_into(comments::table)
        .values(&new_comment)
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}

/// Mark a comment as deleted by setting deleted_date.
pub fn mark_comment_deleted(
    conn: &mut SqliteConnection,
    comment_id: i64,
    deleted_date: i64,
) -> Result<(), diesel::result::Error> {
    use comments::dsl;

    diesel::update(comments::table.filter(dsl::id.eq(comment_id)))
        .set(dsl::deleted_date.eq(Some(deleted_date)))
        .execute(conn)?;
    Ok(())
}

/// Get all active (non-deleted) comments for a steam_id.
pub fn get_active_comments_for_account(
    conn: &mut SqliteConnection,
    steam_id: i64,
) -> Result<Vec<Comment>, diesel::result::Error> {
    use comments::dsl;

    comments::table
        .filter(dsl::steam_id.eq(steam_id))
        .filter(dsl::deleted_date.is_null())
        .load::<Comment>(conn)
}

/// Insert or update a playtime record.
pub fn upsert_playtime(
    conn: &mut SqliteConnection,
    new_playtime: NewPlaytime,
) -> Result<(), diesel::result::Error> {
    use playtime::dsl;

    diesel::insert_into(playtime::table)
        .values(&new_playtime)
        .on_conflict((dsl::steam_id, dsl::game))
        .do_update()
        .set((
            dsl::play_minutes.eq(&new_playtime.play_minutes),
            dsl::last_updated.eq(&new_playtime.last_updated),
        ))
        .execute(conn)?;
    Ok(())
}

/// Update account's friends_fetched timestamp.
/// If the account doesn't exist, creates a minimal placeholder account record
/// that will be updated by the SteamAPI thread later with full details.
pub fn update_account_friends_fetched(
    conn: &mut SqliteConnection,
    steam_id: i64,
    friends_fetched: i64,
) -> Result<(), diesel::result::Error> {
    use account::dsl;

    // Create a minimal account record with placeholder data if it doesn't exist
    let placeholder_account = NewAccount {
        steam_id,
        name: String::from("Unknown"),
        created_date: None,
        avatar_thumb_url: String::new(),
        avatar_full_url: String::new(),
        public_profile: false,
        last_updated: friends_fetched,
        friends_fetched: Some(friends_fetched),
        comments_fetched: None,
        playtimes_fetched: None,
        reputation_fetched: None,
    };

    // Use INSERT OR REPLACE (UPSERT) to either create or update the record
    // on_conflict will update only friends_fetched if account already exists
    diesel::insert_into(account::table)
        .values(&placeholder_account)
        .on_conflict(account::steam_id)
        .do_update()
        .set(dsl::friends_fetched.eq(Some(friends_fetched)))
        .execute(conn)?;
    Ok(())
}

/// Update account's comments_fetched timestamp.
/// If the account doesn't exist, creates a minimal placeholder account record
/// that will be updated by the SteamAPI thread later with full details.
pub fn update_account_comments_fetched(
    conn: &mut SqliteConnection,
    steam_id: i64,
    comments_fetched: i64,
) -> Result<(), diesel::result::Error> {
    use account::dsl;

    // Create a minimal account record with placeholder data if it doesn't exist
    let placeholder_account = NewAccount {
        steam_id,
        name: String::from("Unknown"),
        created_date: None,
        avatar_thumb_url: String::new(),
        avatar_full_url: String::new(),
        public_profile: false,
        last_updated: comments_fetched,
        friends_fetched: None,
        comments_fetched: Some(comments_fetched),
        playtimes_fetched: None,
        reputation_fetched: None,
    };

    // Use INSERT OR REPLACE (UPSERT) to either create or update the record
    // on_conflict will update only comments_fetched if account already exists
    diesel::insert_into(account::table)
        .values(&placeholder_account)
        .on_conflict(account::steam_id)
        .do_update()
        .set(dsl::comments_fetched.eq(Some(comments_fetched)))
        .execute(conn)?;
    Ok(())
}

/// Update account's playtimes_fetched timestamp.
/// If the account doesn't exist, creates a minimal placeholder account record
/// that will be updated by the SteamAPI thread later with full details.
pub fn update_account_playtimes_fetched(
    conn: &mut SqliteConnection,
    steam_id: i64,
    playtimes_fetched: i64,
) -> Result<(), diesel::result::Error> {
    use account::dsl;

    // Create a minimal account record with placeholder data if it doesn't exist
    let placeholder_account = NewAccount {
        steam_id,
        name: String::from("Unknown"),
        created_date: None,
        avatar_thumb_url: String::new(),
        avatar_full_url: String::new(),
        public_profile: false,
        last_updated: playtimes_fetched,
        friends_fetched: None,
        comments_fetched: None,
        playtimes_fetched: Some(playtimes_fetched),
        reputation_fetched: None,
    };

    // Use INSERT OR REPLACE (UPSERT) to either create or update the record
    // on_conflict will update only playtimes_fetched if account already exists
    diesel::insert_into(account::table)
        .values(&placeholder_account)
        .on_conflict(account::steam_id)
        .do_update()
        .set(dsl::playtimes_fetched.eq(Some(playtimes_fetched)))
        .execute(conn)?;
    Ok(())
}

/// Update account's reputation_fetched timestamp.
/// If the account doesn't exist, creates a minimal placeholder account record
/// that will be updated by the SteamAPI thread later with full details.
pub fn update_account_reputation_fetched(
    conn: &mut SqliteConnection,
    steam_id: i64,
    reputation_fetched: i64,
) -> Result<(), diesel::result::Error> {
    use account::dsl;

    // Create a minimal account record with placeholder data if it doesn't exist
    let placeholder_account = NewAccount {
        steam_id,
        name: String::from("Unknown"),
        created_date: None,
        avatar_thumb_url: String::new(),
        avatar_full_url: String::new(),
        public_profile: false,
        last_updated: reputation_fetched,
        friends_fetched: None,
        comments_fetched: None,
        playtimes_fetched: None,
        reputation_fetched: Some(reputation_fetched),
    };

    // Use INSERT OR REPLACE (UPSERT) to either create or update the record
    // on_conflict will update only reputation_fetched if account already exists
    diesel::insert_into(account::table)
        .values(&placeholder_account)
        .on_conflict(account::steam_id)
        .do_update()
        .set(dsl::reputation_fetched.eq(Some(reputation_fetched)))
        .execute(conn)?;
    Ok(())
}

/// Get playtime for a specific steam_id and game.
pub fn get_playtime(
    conn: &mut SqliteConnection,
    steam_id: i64,
    game: Game,
) -> Result<Option<super::entities::Playtime>, diesel::result::Error> {
    use playtime::dsl;

    playtime::table
        .filter(dsl::steam_id.eq(steam_id))
        .filter(dsl::game.eq(game))
        .first::<super::entities::Playtime>(conn)
        .optional()
}

// ============================================================================
// Ban-related queries
// ============================================================================

/// Insert a new ban record.
/// Does not update if already exists (bans are immutable once created).
pub fn insert_ban(
    conn: &mut SqliteConnection,
    new_ban: NewBan,
) -> Result<(), diesel::result::Error> {
    diesel::insert_into(bans::table)
        .values(&new_ban)
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}

/// Get all active (non-expired) bans for a steam_id.
pub fn get_active_bans_for_account(
    conn: &mut SqliteConnection,
    steam_id: i64,
    current_time: i64,
) -> Result<Vec<Ban>, diesel::result::Error> {
    use bans::dsl;

    bans::table
        .filter(dsl::steam_id.eq(steam_id))
        .filter(
            dsl::permanent
                .eq(true)
                .or(dsl::expires_date.is_null())
                .or(dsl::expires_date.gt(current_time)),
        )
        .load::<Ban>(conn)
}

/// Get all bans for a steam_id (including expired).
pub fn get_all_bans_for_account(
    conn: &mut SqliteConnection,
    steam_id: i64,
) -> Result<Vec<Ban>, diesel::result::Error> {
    use bans::dsl;

    bans::table
        .filter(dsl::steam_id.eq(steam_id))
        .load::<Ban>(conn)
}

// ============================================================================
// Ban source-related queries
// ============================================================================

/// Insert or update a ban source.
/// If the source already exists, updates url, parser, and active status.
pub fn upsert_ban_source(
    conn: &mut SqliteConnection,
    new_source: NewBanSource,
) -> Result<(), diesel::result::Error> {
    use ban_sources::dsl;

    diesel::insert_into(ban_sources::table)
        .values(&new_source)
        .on_conflict(dsl::name)
        .do_update()
        .set((
            dsl::url.eq(&new_source.url),
            dsl::parser.eq(&new_source.parser),
            dsl::active.eq(&new_source.active),
        ))
        .execute(conn)?;
    Ok(())
}

/// Update the last_checked timestamp for a ban source.
pub fn update_ban_source_last_checked(
    conn: &mut SqliteConnection,
    name: &str,
    last_checked: i64,
) -> Result<(), diesel::result::Error> {
    use ban_sources::dsl;

    diesel::update(ban_sources::table.filter(dsl::name.eq(name)))
        .set(dsl::last_checked.eq(Some(last_checked)))
        .execute(conn)?;
    Ok(())
}

/// Get all active ban sources.
pub fn get_active_ban_sources(
    conn: &mut SqliteConnection,
) -> Result<Vec<BanSource>, diesel::result::Error> {
    use ban_sources::dsl;

    ban_sources::table
        .filter(dsl::active.eq(true))
        .load::<BanSource>(conn)
}

/// Get all ban sources (active and inactive).
pub fn get_all_ban_sources(
    conn: &mut SqliteConnection,
) -> Result<Vec<BanSource>, diesel::result::Error> {
    ban_sources::table.load::<BanSource>(conn)
}

/// Enable or disable a ban source.
pub fn set_ban_source_active(
    conn: &mut SqliteConnection,
    name: &str,
    active: bool,
) -> Result<(), diesel::result::Error> {
    use ban_sources::dsl;

    diesel::update(ban_sources::table.filter(dsl::name.eq(name)))
        .set(dsl::active.eq(active))
        .execute(conn)?;
    Ok(())
}

// ============================================================================
// Player flag-related queries
// ============================================================================

/// Insert or update a player flag.
/// If the flag already exists, updates last_seen timestamp.
pub fn upsert_player_flag(
    conn: &mut SqliteConnection,
    new_flag: NewPlayerFlag,
) -> Result<(), diesel::result::Error> {
    use player_flags::dsl;

    diesel::insert_into(player_flags::table)
        .values(&new_flag)
        .on_conflict((dsl::steam_id, dsl::flag_type, dsl::source))
        .do_update()
        .set(dsl::last_seen.eq(&new_flag.last_seen))
        .execute(conn)?;
    Ok(())
}

/// Mark a player flag as notified.
pub fn mark_player_flag_notified(
    conn: &mut SqliteConnection,
    steam_id: i64,
    flag_type: &str,
    source: &str,
) -> Result<(), diesel::result::Error> {
    use player_flags::dsl;

    diesel::update(
        player_flags::table.filter(
            dsl::steam_id
                .eq(steam_id)
                .and(dsl::flag_type.eq(flag_type))
                .and(dsl::source.eq(source)),
        ),
    )
    .set(dsl::notified.eq(true))
    .execute(conn)?;
    Ok(())
}

/// Get all player flags for a steam_id.
pub fn get_player_flags(
    conn: &mut SqliteConnection,
    steam_id: i64,
) -> Result<Vec<PlayerFlag>, diesel::result::Error> {
    use player_flags::dsl;

    player_flags::table
        .filter(dsl::steam_id.eq(steam_id))
        .load::<PlayerFlag>(conn)
}

/// Get unnotified player flags for a steam_id.
pub fn get_unnotified_player_flags(
    conn: &mut SqliteConnection,
    steam_id: i64,
) -> Result<Vec<PlayerFlag>, diesel::result::Error> {
    use player_flags::dsl;

    player_flags::table
        .filter(dsl::steam_id.eq(steam_id))
        .filter(dsl::notified.eq(false))
        .load::<PlayerFlag>(conn)
}

/// Remove a player flag.
pub fn remove_player_flag(
    conn: &mut SqliteConnection,
    steam_id: i64,
    flag_type: &str,
    source: &str,
) -> Result<(), diesel::result::Error> {
    use player_flags::dsl;

    diesel::delete(
        player_flags::table.filter(
            dsl::steam_id
                .eq(steam_id)
                .and(dsl::flag_type.eq(flag_type))
                .and(dsl::source.eq(source)),
        ),
    )
    .execute(conn)?;
    Ok(())
}
