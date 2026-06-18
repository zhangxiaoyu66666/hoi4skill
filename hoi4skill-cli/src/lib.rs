//! Public library surface for the hoi4skill CLI.
//!
//! The binary is intentionally thin. Domain code is split into modules so HOI4
//! authoring can keep growing without returning to a single-file executable, and
//! the Clausewitz profile layer gives future games a place to plug in their own
//! directory, event, and syntax conventions.

mod args;
mod cards;
mod clausewitz;
mod clausewitz_library;
mod clausewitz_script;
pub mod cli;
mod copy_prompt;
mod detect_path;
mod edit_context;
pub mod error;
mod error_log;
mod event_cards;
mod feature_cards;
mod focus_excel;
mod focus_layout;
mod game_index;
mod gfx_audit;
mod history_plan;
mod hoi4yaml;
mod icons;
mod import_ir;
mod install_doctor;
mod io_util;
mod json;
mod large_mod;
mod loc_audit;
mod localisation_translate;
mod logic_audit;
mod mod_index;
mod mod_scan;
mod prelude;
mod reference_table;
mod scaffold;
mod scan;
mod text_alignment;
mod usage;
mod util;
mod validate;
mod workflow;

pub(crate) use args::*;
pub(crate) use cards::*;
pub(crate) use clausewitz::*;
pub(crate) use clausewitz_library::*;
pub(crate) use clausewitz_script::*;
pub(crate) use copy_prompt::*;
pub(crate) use detect_path::*;
pub(crate) use edit_context::*;
pub(crate) use error_log::*;
pub(crate) use event_cards::*;
pub(crate) use feature_cards::*;
pub(crate) use focus_excel::*;
pub(crate) use focus_layout::*;
pub(crate) use game_index::*;
pub(crate) use gfx_audit::*;
pub(crate) use history_plan::*;
pub(crate) use hoi4yaml::*;
pub(crate) use icons::*;
pub(crate) use import_ir::*;
pub(crate) use install_doctor::*;
pub(crate) use io_util::*;
pub(crate) use json::*;
pub(crate) use large_mod::*;
pub(crate) use loc_audit::*;
pub(crate) use localisation_translate::*;
pub(crate) use logic_audit::*;
pub(crate) use mod_index::*;
pub(crate) use mod_scan::*;
pub(crate) use prelude::*;
pub(crate) use reference_table::*;
pub(crate) use scaffold::*;
pub(crate) use scan::*;
pub(crate) use text_alignment::*;
pub(crate) use usage::*;
pub(crate) use util::*;
pub(crate) use validate::*;
pub(crate) use workflow::*;

#[cfg(test)]
mod tests;
