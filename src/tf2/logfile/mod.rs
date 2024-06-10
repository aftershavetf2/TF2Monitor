pub mod line_parser;
pub mod logfile_watcher;

use chrono::prelude::*;

///  This type covers the different types of log lines that can be parsed from the log file.
/// It's unprocessed data mostly, so it's names instead of SteamIDs.
#[derive(Debug, PartialEq, Clone)]
pub enum LogLine {
    /// The output of the status command. The first line is the header, the following lines are player data.
    // 05/08/2024 - 14:25:11: # userid name                uniqueid            connected ping loss state
    StatusHeader {
        /// Local time
        when: DateTime<Local>,
    },

    /// 05/06/2024 - 17:15:24: #   2802 "Holy"              [U:1:169802]     34:11       56    0 active
    StatusForPlayer {
        /// Local time
        when: DateTime<Local>,

        // The player's id in the server.
        id: u32,

        // The player's name.
        name: String,

        /// The player's SteamID32 as a string.
        steam_id32: String,
    },

    /// The output of tf_lobby_debug command
    //   Member[1] [U:1:169802]  team = TF_GC_TEAM_DEFENDERS  type = MATCH_PLAYER
    PlayerTeam { steam_id32: String, team: String },

    /// A player killed another player with a weapon, possibly a crit.
    /// Example:
    /// 05/06/2024 - 17:02:55: Player1 killed Player2 with iron_bomber. (crit)
    Kill {
        /// Local time
        when: DateTime<Local>,
        killer: String,
        victim: String,
        weapon: String,
        crit: bool,
    },

    Suicide {
        /// Local time
        when: DateTime<Local>,
        name: String,
    },
    /*
    05/06/2024 - 17:05:50: hostname: Valve Matchmaking Server (Frankfurt srcds101-fra2 #70)
    05/06/2024 - 17:05:50: version : 8835751/24 8835751 secure
    05/06/2024 - 17:05:50: udp/ip  : 169.254.171.56:28664
    05/06/2024 - 17:05:50: steamid : [A:1:509071377:29317] (90197908612698129)
    05/06/2024 - 17:05:50: account : not logged in  (No account specified)
    05/06/2024 - 17:05:50: map     : pl_badwater at: 0 x, 0 y, 0 z
    05/06/2024 - 17:05:50: tags    : hidden,increased_maxplayers,payload,valve
    05/06/2024 - 17:05:50: players : 24 humans, 0 bots (32 max)
    05/06/2024 - 17:05:50: edicts  : 959 used of 2048 max
         */
    // 05/06/2024 - 17:05:58: Benito Tortellini (real), Slightly Destructive Justice, TopG_14 captured Third Capture point for team #3
    LobbyCreated {
        /// Local time
        when: DateTime<Local>,
    },

    LobbyDestroyed {
        /// Local time
        when: DateTime<Local>,
    },

    Chat {
        /// Local time
        when: DateTime<Local>,
        name: String,
        message: String,
        dead: bool,
        team: bool,
    },
}
