//! Shared imports for the internal modules.
//!
//! The crate is split by domain, but many legacy functions still share common
//! standard-library types while the deeper API boundaries are being tightened.

pub(crate) use std::collections::{BTreeMap, BTreeSet, HashMap};
pub(crate) use std::env;
pub(crate) use std::ffi::OsStr;
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};
