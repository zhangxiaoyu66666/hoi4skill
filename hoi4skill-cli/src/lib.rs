//! Public library surface for the hoi4skill CLI.
//!
//! The binary is intentionally thin. Domain code is split into modules so HOI4
//! authoring can keep growing without returning to a single-file executable, and
//! the Clausewitz profile layer gives future games a place to plug in their own
//! directory, event, and syntax conventions.

#![allow(
    clippy::if_same_then_else,
    clippy::chars_last_cmp,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::double_ended_iterator_last,
    clippy::len_zero,
    clippy::manual_contains,
    clippy::manual_pattern_char_comparison,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_character_iteration,
    clippy::needless_option_as_deref,
    clippy::overly_complex_bool_expr,
    clippy::question_mark,
    clippy::redundant_closure,
    clippy::single_char_add_str,
    clippy::too_many_arguments,
    clippy::trim_split_whitespace,
    clippy::type_complexity,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_to_owned,
    clippy::unnecessary_unwrap,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::while_let_on_iterator
)]

mod ai_behavior;
mod ambiguity;
mod args;
mod author_compiler;
mod cards;
mod character;
mod clausewitz;
mod clausewitz_library;
mod clausewitz_script;
pub mod cli;
mod common_coverage;
mod common_writers;
mod copy_prompt;
mod core_ir;
mod country_setup;
mod detect_path;
mod diplomacy;
mod documentation;
mod edit_context;
pub mod error;
mod error_log;
mod event_cards;
mod feature_cards;
mod flags;
mod focus_excel;
mod focus_layout;
mod game_index;
mod game_index_cache;
mod gfx_audit;
mod gfx_manifest;
mod gui_layout_audit;
mod history_plan;
mod history_scenario;
mod history_systems;
mod hoi4yaml;
mod icons;
mod ideation;
mod ideology;
mod import_ir;
mod install_doctor;
mod io_util;
mod json;
mod knowledge;
mod large_mod;
mod layered_sources;
mod loc_audit;
mod localisation_glossary;
mod localisation_tokens;
mod localisation_translate;
mod logic_audit;
mod map_data;
mod mod_index;
mod mod_scan;
mod overall_core;
mod parallel_work;
mod parent_mod;
mod phase;
mod prelude;
mod reference_table;
mod route;
mod runtime_gate;
mod runtime_session;
mod safety;
mod scaffold;
mod scan;
mod scope_contract;
mod scope_systems;
mod stale_plan;
mod system_packs;
mod text_alignment;
mod transaction;
mod ui_cosmetic;
mod unit_taxonomy;
mod usage;
mod util;
mod validate;
mod workflow;
mod writer_readiness;

pub(crate) use ai_behavior::*;
pub(crate) use ambiguity::*;
pub(crate) use args::*;
pub(crate) use author_compiler::*;
pub(crate) use cards::*;
pub(crate) use character::*;
pub(crate) use clausewitz::*;
pub(crate) use clausewitz_library::*;
pub(crate) use clausewitz_script::*;
pub(crate) use common_coverage::*;
pub(crate) use common_writers::*;
pub(crate) use copy_prompt::*;
pub(crate) use core_ir::*;
pub(crate) use country_setup::*;
pub(crate) use detect_path::*;
pub(crate) use diplomacy::*;
pub(crate) use documentation::*;
pub(crate) use edit_context::*;
pub(crate) use error_log::*;
pub(crate) use event_cards::*;
pub(crate) use feature_cards::*;
pub(crate) use flags::*;
pub(crate) use focus_excel::*;
pub(crate) use focus_layout::*;
pub(crate) use game_index::*;
pub(crate) use game_index_cache::*;
pub(crate) use gfx_audit::*;
pub(crate) use gfx_manifest::*;
pub(crate) use gui_layout_audit::*;
pub(crate) use history_plan::*;
pub(crate) use history_scenario::*;
pub(crate) use history_systems::*;
pub(crate) use hoi4yaml::*;
pub(crate) use icons::*;
pub(crate) use ideation::*;
pub(crate) use ideology::*;
pub(crate) use import_ir::*;
pub(crate) use install_doctor::*;
pub(crate) use io_util::*;
pub(crate) use json::*;
pub(crate) use knowledge::*;
pub(crate) use large_mod::*;
pub(crate) use layered_sources::*;
pub(crate) use loc_audit::*;
pub(crate) use localisation_glossary::*;
pub(crate) use localisation_tokens::*;
pub(crate) use localisation_translate::*;
pub(crate) use logic_audit::*;
pub(crate) use map_data::*;
pub(crate) use mod_index::*;
pub(crate) use mod_scan::*;
pub(crate) use overall_core::*;
pub(crate) use parallel_work::*;
pub(crate) use parent_mod::*;
pub(crate) use phase::*;
pub(crate) use prelude::*;
pub(crate) use reference_table::*;
pub(crate) use route::*;
pub(crate) use runtime_gate::*;
pub(crate) use runtime_session::*;
pub(crate) use safety::*;
pub(crate) use scaffold::*;
pub(crate) use scan::*;
pub(crate) use scope_contract::*;
pub(crate) use scope_systems::*;
pub(crate) use stale_plan::*;
pub(crate) use system_packs::*;
pub(crate) use text_alignment::*;
pub(crate) use transaction::*;
pub(crate) use ui_cosmetic::*;
pub(crate) use unit_taxonomy::*;
pub(crate) use usage::*;
pub(crate) use util::*;
pub(crate) use validate::*;
pub(crate) use workflow::*;
pub(crate) use writer_readiness::*;

#[cfg(test)]
mod tests;
