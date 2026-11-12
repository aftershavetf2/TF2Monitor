//! Candidate classes for the original weapon identifiers in TF2 kill messages.
//!
//! Sources (checked 2026-11-12): TF2 scripts/items/items_game.txt
//! (`item_logname`, `item_iconname`, `item_class`, inherited `used_by_classes`),
//! https://github.com/SteamTracking/GameTracking-TF2/blob/master/tf/scripts/items/items_game.txt
//! and engine-generated names in Valve's CTFGameRules::GetKillingWeaponName:
//! https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_gamerules.cpp
//! Projectile entity names and shared variants are also checked against Valve's
//! tf_projectile_*.cpp and tf_weapon_jar.cpp implementations.
//! Shared identifiers include all eligible classes, not a guessed single class.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerClass {
    Scout,
    Soldier,
    Pyro,
    Demoman,
    Heavy,
    Engineer,
    Medic,
    Sniper,
    Spy,
}

/// Returns a class only when the kill weapon identifies it unambiguously.
///
/// Expects the original, case-sensitive console identifier (e.g. `iron_bomber`).
/// This is not the player's current class: they may have changed class since.
/// `None` means undecided: unknown identifiers, generic damage, or shared weapons.
/// The table retains candidate sets so ambiguous identifiers cannot be mistaken
/// for class-specific ones.
pub fn weapon_class(weapon: &str) -> Option<PlayerClass> {
    use PlayerClass::*;

    // Entity weapon names use the same identifiers with this prefix. Keep
    // projectile and wearable names intact: they need their own candidate sets.
    let weapon = weapon.strip_prefix("tf_weapon_").unwrap_or(weapon);

    let candidates: &[PlayerClass] = match weapon {
        "ai_flamethrower"
        | "annihilator"
        | "armageddon"
        | "axtinguisher"
        | "back_scratcher"
        | "backburner"
        | "breakable_sign"
        | "deflect_arrow"
        | "deflect_ball"
        | "deflect_flare"
        | "deflect_flare_detonator"
        | "deflect_huntsman_flyingburn"
        | "deflect_promode"
        | "deflect_rocket"
        | "deflect_sticky"
        | "degreaser"
        | "detonator"
        | "dragons_fury"
        | "dragons_fury_bonus"
        | "fireaxe"
        | "flamethrower"
        | "flaregun"
        | "flaregun_revenge"
        | "gas_blast"
        | "hot_hand"
        | "jar_gas"
        | "lava_axe"
        | "lollichop"
        | "loose_cannon_reflect"
        | "mailbox"
        | "manmelter"
        | "phlogistinator"
        | "powerjack"
        | "rainblower"
        | "rescue_ranger_reflect"
        | "rocketlauncher_fireball"
        | "rocketpack"
        | "scorch_shot"
        | "shotgun_pyro"
        | "slap"
        | "sledgehammer"
        | "taunt_pyro"
        | "the_maul"
        | "tf_projectile_balloffire"
        | "tf_projectile_flare"
        | "thirddegree" => &[Pyro],

        "airstrike"
        | "blackbox"
        | "buff_item"
        | "cow_mangler"
        | "disciplinary_action"
        | "dumpster_device"
        | "liberty_launcher"
        | "market_gardener"
        | "particle_cannon"
        | "pickaxe"
        | "quake_rl"
        | "raygun"
        | "righteous_bison"
        | "rocketlauncher"
        | "rocketlauncher_airstrike"
        | "rocketlauncher_directhit"
        | "shotgun_soldier"
        | "shovel"
        | "taunt_soldier"
        | "taunt_soldier_lumbricus"
        | "tf_projectile_energy_ball"
        | "tf_projectile_rocket"
        | "unique_pickaxe"
        | "unique_pickaxe_escape" => &[Soldier],

        "ambassador" | "big_earner" | "black_rose" | "diamondback" | "enforcer"
        | "eternal_reward" | "invis" | "knife" | "kunai" | "letranger" | "pda_spy" | "psapper"
        | "recorder" | "revolver" | "samrevolver" | "sapper" | "sharp_dresser" | "snack_attack"
        | "spy_cicle" | "taunt_spy" | "voodoo_pin" => &[Spy],

        "amputator"
        | "battleneedle"
        | "blutsauger"
        | "bonesaw"
        | "crossbow"
        | "crusaders_crossbow"
        | "medigun"
        | "proto_syringe"
        | "solemn_vow"
        | "syringegun_medic"
        | "taunt_medic"
        | "tf_projectile_healing_bolt"
        | "tf_projectile_syringe"
        | "ubersaw" => &[Medic],

        "apocofists"
        | "brass_beast"
        | "bread_bite"
        | "eviction_notice"
        | "family_business"
        | "fists"
        | "gloves"
        | "gloves_running_urgently"
        | "holiday_punch"
        | "iron_curtain"
        | "long_heatmaker"
        | "lunchbox"
        | "minigun"
        | "natascha"
        | "shotgun_hwg"
        | "steel_fists"
        | "taunt_heavy"
        | "tomislav"
        | "warrior_spirit" => &[Heavy],

        "atomizer"
        | "back_scatter"
        | "bat"
        | "bat_fish"
        | "bat_giftwrap"
        | "bat_wood"
        | "batsaber"
        | "boston_basher"
        | "candy_cane"
        | "cleaver"
        | "force_a_nature"
        | "guillotine"
        | "handgun_scout_primary"
        | "handgun_scout_secondary"
        | "holymackerel"
        | "jar_milk"
        | "lava_bat"
        | "lunchbox_drink"
        | "pep_brawler_blaster"
        | "pep_brawlerblaster"
        | "pep_pistol"
        | "pistol_scout"
        | "sandman"
        | "scattergun"
        | "scout_sword"
        | "shortstop"
        | "soda_popper"
        | "taunt_scout"
        | "tf_projectile_cleaver"
        | "tf_projectile_jar_milk"
        | "the_winger"
        | "unarmed_combat"
        | "warfan"
        | "wrap_assassin" => &[Scout],

        "awper_hand"
        | "bazaar_bargain"
        | "bushwacka"
        | "charged_smg"
        | "club"
        | "compound_bow"
        | "huntsman"
        | "jar"
        | "machina"
        | "player_penetration"
        | "pro_rifle"
        | "pro_smg"
        | "shahanshah"
        | "shooting_star"
        | "smg"
        | "sniperrifle"
        | "sniperrifle_classic"
        | "sniperrifle_decap"
        | "sydney_sleeper"
        | "taunt_sniper"
        | "tf_projectile_jar"
        | "the_classic"
        | "tribalkukri" => &[Sniper],

        "battleaxe"
        | "bottle"
        | "cannon"
        | "claidheamohmor"
        | "demoshield"
        | "grenadelauncher"
        | "headtaker"
        | "iron_bomber"
        | "loch_n_load"
        | "loose_cannon"
        | "loose_cannon_explosion"
        | "loose_cannon_impact"
        | "nessieclub"
        | "persian_persuader"
        | "pipebomblauncher"
        | "quickiebomb_launcher"
        | "scotland_shard"
        | "splendid_screen"
        | "stickbomb"
        | "sticky_resistance"
        | "stickybomb_defender"
        | "sword"
        | "taunt_demoman"
        | "tf_projectile_pipe"
        | "tf_projectile_pipe_remote"
        | "tf_wearable_demoshield"
        | "tide_turner"
        | "ullapool_caber"
        | "ullapool_caber_explosion" => &[Demoman],

        "drg_pomson"
        | "eureka_effect"
        | "frontier_justice"
        | "frontier_kill"
        | "giger_counter"
        | "laser_pointer"
        | "mechanical_arm"
        | "obj_minisentry"
        | "obj_sentrygun"
        | "obj_sentrygun2"
        | "obj_sentrygun3"
        | "pda_engineer_build"
        | "pda_engineer_destroy"
        | "pomson"
        | "rescue_ranger"
        | "robot_arm"
        | "robot_arm_blender_kill"
        | "robot_arm_combo_kill"
        | "robot_arm_kill"
        | "sentry_revenge"
        | "short_circuit"
        | "shotgun_building_rescue"
        | "shotgun_primary"
        | "southern_comfort_kill"
        | "southern_hospitality"
        | "taunt_guitar_kill"
        | "taunt_guitar_riff_kill"
        | "telefrag"
        | "tf_projectile_sentryrocket"
        | "widowmaker"
        | "wrangler_kill"
        | "wrench"
        | "wrench_golden"
        | "wrench_jag" => &[Engineer],

        "builder" => &[Engineer, Spy],

        // Rescue Ranger repair bolts also use the arrow entity. Energy rings
        // are shared by the Righteous Bison and Pomson 6000.
        "tf_projectile_arrow" => &[Sniper, Engineer],
        "tf_projectile_energy_ring" => &[Soldier, Engineer],

        "demokatana" | "katana" | "paintrain" | "parachute" => &[Soldier, Demoman],

        "maxgun" | "pistol" | "the_capper" => &[Scout, Engineer],

        "reserve_shooter" => &[Soldier, Pyro],

        "panic_attack" | "shotgun" => &[Soldier, Pyro, Heavy, Engineer],

        "crossing_guard"
        | "freedom_staff"
        | "fryingpan"
        | "ham_shank"
        | "memory_maker"
        | "nonnonviolent_protest"
        | "skullbat" => &[Scout, Soldier, Pyro, Demoman, Heavy, Medic, Sniper],

        "necro_smasher" => &[
            Scout, Soldier, Pyro, Demoman, Heavy, Engineer, Medic, Sniper,
        ],

        "golden_fryingpan"
        | "grappling_hook"
        | "grapplinghook"
        | "passtime_gun"
        | "prinny_machete"
        | "saxxy"
        | "spellbook"
        | "spellbook_athletic"
        | "spellbook_bats"
        | "spellbook_blastjump"
        | "spellbook_boss"
        | "spellbook_fireball"
        | "spellbook_lightning"
        | "spellbook_meteor"
        | "spellbook_mirv"
        | "spellbook_skeleton"
        | "spellbook_teleport"
        | "tf_projectile_grapplinghook" => &[
            Scout, Soldier, Pyro, Demoman, Heavy, Engineer, Medic, Sniper, Spy,
        ],
        _ => &[],
    };
    match candidates {
        [class] => Some(*class),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{PlayerClass::*, *};

    #[test]
    fn identifies_each_class_from_a_unique_weapon() {
        for (weapon, class) in [
            ("scattergun", Scout),
            ("rocketlauncher", Soldier),
            ("flamethrower", Pyro),
            ("iron_bomber", Demoman),
            ("minigun", Heavy),
            ("wrench", Engineer),
            ("crusaders_crossbow", Medic),
            ("sniperrifle", Sniper),
            ("knife", Spy),
        ] {
            assert_eq!(weapon_class(weapon), Some(class), "{weapon}");
        }
    }

    #[test]
    fn preserves_shared_weapon_ambiguity() {
        assert!(weapon_class("shotgun").is_none());
        assert_eq!(weapon_class("pistol"), None);
        assert!(weapon_class("panic_attack").is_none());
        assert_eq!(weapon_class("demokatana"), None);
        assert!(weapon_class("fryingpan").is_none());
        assert!(weapon_class("golden_fryingpan").is_none());
        assert!(weapon_class("saxxy").is_none());
    }

    #[test]
    fn identifies_prefixed_weapon_entities_for_every_class() {
        for (weapon, class) in [
            ("tf_weapon_scattergun", Scout),
            ("tf_weapon_rocketlauncher", Soldier),
            ("tf_weapon_rocketlauncher_fireball", Pyro),
            ("tf_weapon_grenadelauncher", Demoman),
            ("tf_weapon_minigun", Heavy),
            ("tf_weapon_shotgun_primary", Engineer),
            ("tf_weapon_syringegun_medic", Medic),
            ("tf_weapon_sniperrifle", Sniper),
            ("tf_weapon_revolver", Spy),
        ] {
            assert_eq!(weapon_class(weapon), Some(class), "{weapon}");
        }
    }

    #[test]
    fn identifies_additional_projectile_entities() {
        for (weapon, class) in [
            ("tf_projectile_balloffire", Pyro),
            ("tf_projectile_flare", Pyro),
            ("tf_projectile_energy_ball", Soldier),
            ("tf_projectile_healing_bolt", Medic),
            ("tf_projectile_syringe", Medic),
            ("tf_projectile_cleaver", Scout),
            ("tf_projectile_jar_milk", Scout),
            ("tf_projectile_jar", Sniper),
            ("tf_projectile_sentryrocket", Engineer),
            ("tf_wearable_demoshield", Demoman),
        ] {
            assert_eq!(weapon_class(weapon), Some(class), "{weapon}");
        }
    }

    #[test]
    fn leaves_shared_entities_and_unknown_prefixed_names_unclassified() {
        for weapon in [
            "tf_weapon_pistol",
            "tf_weapon_shotgun",
            "tf_weapon_builder",
            "tf_weapon_katana",
            "tf_weapon_grapplinghook",
            "tf_projectile_arrow",
            "tf_projectile_energy_ring",
            "tf_projectile_grapplinghook",
            "tf_weapon_bleed_kill",
            "tf_weapon_custom_weapon",
            "tf_weapon_",
            "tf_rocketlauncher",
            "tf_projectile_custom_weapon",
            "tf_wearable",
        ] {
            assert_eq!(weapon_class(weapon), None, "{weapon}");
        }
    }

    #[test]
    fn handles_engine_names_and_reflected_projectiles() {
        for weapon in [
            "obj_sentrygun",
            "obj_sentrygun2",
            "obj_sentrygun3",
            "obj_minisentry",
        ] {
            assert_eq!(weapon_class(weapon), Some(Engineer));
        }
        for weapon in ["deflect_rocket", "deflect_arrow", "loose_cannon_reflect"] {
            assert_eq!(weapon_class(weapon), Some(Pyro));
        }
        assert_eq!(weapon_class("shotgun_soldier"), Some(Soldier));
        assert_eq!(weapon_class("tf_projectile_rocket"), Some(Soldier));
        assert_eq!(weapon_class("tf_projectile_pipe_remote"), Some(Demoman));
        assert_eq!(weapon_class("shovel"), Some(Soldier));
    }

    #[test]
    fn leaves_unknown_names_and_generic_damage_unclassified() {
        for weapon in [
            "",
            "world",
            "bleed_kill",
            "custom_weapon",
            "iron bomber",
            "IRON_BOMBER",
        ] {
            assert!(weapon_class(weapon).is_none(), "{weapon}");
        }
    }
}
