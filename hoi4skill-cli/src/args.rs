//! Small argument map parser shared by all command modules.

#[allow(unused_imports)]
use crate::*;

pub(crate) struct ArgMap {
    pub(crate) flags: BTreeSet<String>,
    pub(crate) values: HashMap<String, String>,
    pub(crate) value_lists: HashMap<String, Vec<String>>,
    pub(crate) positionals: Vec<String>,
}

pub(crate) fn parse_args(args: &[String]) -> ArgMap {
    let mut flags = BTreeSet::new();
    let mut values = HashMap::new();
    let mut value_lists: HashMap<String, Vec<String>> = HashMap::new();
    let mut positionals = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(key) = arg.strip_prefix("--") {
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                let key = key.to_string();
                let value = args[i + 1].clone();
                values.insert(key.clone(), value.clone());
                value_lists.entry(key).or_default().push(value);
                i += 2;
            } else {
                flags.insert(key.to_string());
                i += 1;
            }
        } else {
            positionals.push(arg.clone());
            i += 1;
        }
    }
    ArgMap {
        flags,
        values,
        value_lists,
        positionals,
    }
}

pub(crate) fn value<'a>(map: &'a ArgMap, key: &str) -> Option<&'a str> {
    map.values.get(key).map(|s| s.as_str())
}

pub(crate) fn repeated_values<'a>(map: &'a ArgMap, key: &str) -> Vec<&'a str> {
    map.value_lists
        .get(key)
        .map(|values| values.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

pub(crate) fn parse_usize_option(map: &ArgMap, key: &str, default: usize) -> Result<usize, String> {
    value(map, key)
        .map(|raw| {
            raw.parse::<usize>()
                .map_err(|_| format!("--{key} expects a non-negative integer, got {raw}"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

pub(crate) fn require_value(map: &ArgMap, key: &str) -> Result<String, String> {
    value(map, key)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing --{key}"))
}
