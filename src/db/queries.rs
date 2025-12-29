use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, DbErr};

use super::entities::friendship;

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
/// * `db` - Database connection
/// * `steam_id` - The SteamID64 to get friendships for
/// * `active_only` -
///   If `true`, returns only active friendships (where unfriend_date is NULL).
///   If `false`, returns both active and inactive friendships.
///
/// To extract the friend's steam_id from the result:
/// - For direct friendships: use `friendship.friend_steam_id`
/// - For reverse friendships: use `friendship.steam_id`
pub async fn get_friendships(
    db: &DatabaseConnection,
    steam_id: i64,
    active_only: bool,
) -> Result<Vec<friendship::Model>, DbErr> {
    use friendship::Column as FriendshipColumn;
    use friendship::Entity as Friendship;

    // Build query for direct friendships: where steam_id = given_steam_id
    let mut direct_query = Friendship::find().filter(FriendshipColumn::SteamId.eq(steam_id));
    if active_only {
        direct_query = direct_query.filter(FriendshipColumn::UnfriendDate.is_null());
    }
    let direct_friendships = direct_query.all(db).await?;

    // Build query for reverse friendships: where friend_steam_id = given_steam_id
    // These represent accounts that have the given steam_id as a friend
    let mut reverse_query = Friendship::find().filter(FriendshipColumn::FriendSteamId.eq(steam_id));
    if active_only {
        reverse_query = reverse_query.filter(FriendshipColumn::UnfriendDate.is_null());
    }
    let reverse_friendships = reverse_query.all(db).await?;

    // Combine both lists
    // Note: We might get duplicates if a friendship exists in both directions,
    // but that's fine since the composite PK ensures uniqueness
    let mut all_friendships = direct_friendships;
    all_friendships.extend(reverse_friendships);

    Ok(all_friendships)
}
