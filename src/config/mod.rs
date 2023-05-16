use std::{collections::HashMap, fs::OpenOptions};

use self::types::{deserialize_KeyModifiers, deserialize_Keysym, XkbConfig};
use serde::Deserialize;

mod types;
#[derive(Debug, Deserialize)]
pub struct Config {
    pub workspaces: u8,
    pub keybindings: HashMap<KeyPattern, Action>,

    #[serde(default = "default_gaps")]
    pub gaps: (i32, i32),

    #[serde(default = "default_xkb")]
    pub xkb: XkbConfig,

    #[serde(default = "default_autostart")]
    pub autostart: Vec<String>,
}

pub fn load_config() -> Config {
    let xdg = xdg::BaseDirectories::new().ok();
    let locations = if let Some(base) = xdg {
        vec![
            base.get_config_file("magmawm.ron"),
            base.get_config_file("magmawm/config.ron"),
        ]
    } else {
        vec![]
    };

    for path in locations {
        dbg!("Trying config location: {}", path.display());
        if path.exists() {
            dbg!("Using config at {}", path.display());
            return ron::de::from_reader(OpenOptions::new().read(true).open(path).unwrap())
                .expect("Malformed config file");
        }
    }
    panic!("No config file found")
}

fn default_gaps() -> (i32, i32) {
    (5, 5)
}

fn default_xkb() -> XkbConfig {
    XkbConfig::default()
}

fn default_autostart() -> Vec<String> {
    vec![]
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum KeyModifier {
    Ctrl,
    Alt,
    Shift,
    Super,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    ctrl: bool,
    alt: bool,
    shift: bool,
    logo: bool,
}

/// Describtion of a key combination that might be
/// handled by the compositor.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Hash)]
#[serde(deny_unknown_fields)]
pub struct KeyPattern {
    /// What modifiers are expected to be pressed alongside the key
    #[serde(deserialize_with = "deserialize_KeyModifiers")]
    pub modifiers: KeyModifiers,
    /// The actual key, that was pressed
    #[serde(deserialize_with = "deserialize_Keysym")]
    pub key: u32,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    Debug,
    Close,
    Workspace(u8),
    MoveWindow(u8),
    MoveAndSwitch(u8),
    ToggleWindowFloating,
    VTSwitch(i32),
    Spawn(String),
}
