//! Game-profile abstractions for Clausewitz-engine tools.
//!
//! HOI4 is the first supported game, but command code should depend on this
//! profile shape where possible so CK3/EU4/Victoria-style support can add a new
//! profile instead of forking the whole CLI.

/// Data needed by generators and validators that varies between Clausewitz games.
pub struct ClausewitzGameProfile {
    /// Stable machine name for reports and future dispatch.
    pub id: &'static str,
    /// Human-readable game name for diagnostics.
    pub display_name: &'static str,
    /// Event id limit when the game uses namespace-number event ids.
    pub event_id_max: Option<i64>,
    /// Top-level directories whose `.txt` files participate in executable
    /// Clausewitz gameplay logic.
    ///
    /// Keeping this in the game profile avoids teaching every audit its own
    /// directory list and, importantly, prevents script-only audits from
    /// traversing large asset trees such as `gfx`.
    pub script_roots: &'static [&'static str],
}

pub const HOI4_EVENT_ID_MAX: i64 = 200_000;

pub const HOI4_PROFILE: ClausewitzGameProfile = ClausewitzGameProfile {
    id: "hoi4",
    display_name: "Hearts of Iron IV",
    event_id_max: Some(HOI4_EVENT_ID_MAX),
    script_roots: &["common", "events", "history"],
};

pub(crate) const EVENT_ID_MAX: i64 = HOI4_EVENT_ID_MAX;

/// Event id ceiling for the active profile.
///
/// Some Clausewitz games do not use HOI4-style `namespace.number` event ids; the
/// fallback keeps the HOI4 CLI behavior stable while new profiles get introduced.
pub(crate) fn active_event_id_max() -> i64 {
    HOI4_PROFILE.event_id_max.unwrap_or(EVENT_ID_MAX)
}
