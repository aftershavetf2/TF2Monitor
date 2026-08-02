pub mod line_parser;
pub mod logfile_watcher;

///  This type covers the different types of log lines that can be parsed from the log file.
/// It's unprocessed data mostly, so it's names instead of SteamIDs.
#[derive(Debug, PartialEq, Clone)]
pub enum LogLine {
    /// A player killed another player with a weapon, possibly a crit.
    /// Example:
    /// Player1 killed Player2 with iron_bomber. (crit)
    Kill {
        killer: String,
        victim: String,
        weapon: String,
        crit: bool,
    },

    Suicide {
        name: String,
    },
    /*
    hostname: Valve Matchmaking Server (Frankfurt srcds101-fra2 #70)
    version : 8835751/24 8835751 secure
    udp/ip  : 169.254.171.56:28664
    steamid : [A:1:509071377:29317] (90197908612698129)
    account : not logged in  (No account specified)
    map     : pl_badwater at: 0 x, 0 y, 0 z
    tags    : hidden,increased_maxplayers,payload,valve
    players : 24 humans, 0 bots (32 max)
    edicts  : 959 used of 2048 max
         */
    // Benito Tortellini (real), Slightly Destructive Justice, TopG_14 captured Third Capture point for team #3
    LobbyCreated,

    LobbyDestroyed,

    Chat {
        name: String,
        message: String,
        dead: bool,
        team: bool,
    },
}
