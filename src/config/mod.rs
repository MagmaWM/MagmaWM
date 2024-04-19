use std::{fs::File, fs::OpenOptions, io::Write, path::PathBuf};

use self::types::{
    deserialize_EndColour, deserialize_KeyModifiers, deserialize_Keysym, deserialize_StartColour,
    serialize_EndColour, serialize_KeyModifiers, serialize_Keysym, serialize_StartColour,
    XkbConfig,
};
use crate::config::types::KeyModifiersDef;
use indexmap::IndexMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use smithay::{
    input::keyboard::{xkb::keysyms, Keysym},
    utils::{Physical, Size},
};
use tracing::{debug, error, info, warn};

mod types;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub workspaces: u8,
    pub keybindings: IndexMap<KeyPattern, Action>,

    #[serde(default = "default_gaps")]
    pub gaps: (i32, i32),

    #[serde(default = "default_xkb")]
    pub xkb: XkbConfig,

    #[serde(default = "default_autostart")]
    pub autostart: Vec<String>,

    #[serde(default = "default_outputs")]
    pub outputs: IndexMap<String, OutputConfig>,

    #[serde(default = "default_borders")]
    pub borders: Borders,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OutputConfig((i32, i32), Option<u32>);

impl OutputConfig {
    pub fn mode_size(&self) -> Size<i32, Physical> {
        self.0.into()
    }

    pub fn mode_refresh(&self) -> u32 {
        self.1.unwrap_or(60) * 1000
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

/// Description of a key combination that might be handled by the compositor
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Hash, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyPattern {
    /// What modifiers are expected to be pressed alongside the key
    #[serde(deserialize_with = "deserialize_KeyModifiers")]
    #[serde(serialize_with = "serialize_KeyModifiers")]
    pub modifiers: KeyModifiers,
    /// The actual key, that was pressed
    #[serde(deserialize_with = "deserialize_Keysym")]
    #[serde(serialize_with = "serialize_Keysym")]
    pub key: Keysym,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct Borders {
    pub thickness: u8,
    #[serde(deserialize_with = "deserialize_StartColour")]
    #[serde(serialize_with = "serialize_StartColour")]
    pub start_color: [f32; 3],
    #[serde(deserialize_with = "deserialize_EndColour")]
    #[serde(serialize_with = "serialize_EndColour")]
    pub end_color: Option<[f32; 3]>,
    pub radius: f32,
    pub gradient_angle: f32,
}

/// Loads a config file if one exists, otherwise generates a default config.
///
/// The default config file is generated in the `$XDG_CONFIG_HOME/magmawm` directory.
///
/// # Panics
/// - Panics if unable to open or read any of the potential config files.
/// - Panics if the loaded config file is malformed.
/// - Panics if unable to determine the XDG home directory.
/// - Panics if unable to create the `$XDG_CONFIG_HOME/magmawm` directory.
/// - Panics if unable to create or write to the default config file.
/// - Panics if unable to serialize the default config.
///
/// # Returns
/// The config parsed from the loaded or generated config file
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

    // try to read from each config file location
    for path in locations {
        debug!("Checking for config file at '{}'", path.display());
        if !path.exists() {
            continue;
        }
        debug!("Reading config file: '{}'", path.display());
        let file = match OpenOptions::new().read(true).open(&path) {
            Ok(file) => file,
            Err(e) => {
                warn!("Unable to read config file '{}': {e}", path.display());
                continue;
            }
        };
        debug!("Deserializing config file: '{}'", path.display());
        let config_data = match ron::de::from_reader(file) {
            Ok(config_data) => config_data,
            Err(e) => {
                error!("Malformed config file '{}': {e}", path.display());
                continue;
            }
        };
        info!("Successfully loaded config file: '{}'", path.display());
        return config_data;
    }

    warn!("No working config file found");
    let config_path = generate_config();

    debug!("Reading generated config file: '{}'", config_path.display());

    let config_file = OpenOptions::new()
        .read(true)
        .open(&config_path)
        .unwrap_or_else(|e| panic!("Unable to open file '{}': {e}", config_path.display()));

    debug!(
        "Deserializing generated config file: '{}'",
        config_path.display()
    );
    ron::de::from_reader(config_file)
        .unwrap_or_else(|e| panic!("Generated config file is malformed: {e}"))
}

/// Generates a default config file `config.ron` in the `$XDG_CONFIG_HOME/magmawm` directory.
///
/// # Panics
/// - Panics if unable to determine the XDG home directory.
/// - Panics if unable to create the `$XDG_CONFIG_HOME/magmawm` directory.
/// - Panics if unable to create or write to the config file.
/// - Panics if unable to serialize the default config.
///
/// # Returns
/// The path of the generated config file
fn generate_config() -> PathBuf {
    debug!("Loading XDG base directories");
    let xdg = xdg::BaseDirectories::new()
        .unwrap_or_else(|e| panic!("Unable to get XDG base directories: {e}"));
    debug!("Ensuring the config directory exists");
    let config_path = xdg
        .place_config_file("magmawm/config.ron")
        .unwrap_or_else(|e| panic!("Unable to create config directory: {e}"));
    debug!("Creating config file: '{}'", config_path.display());
    let mut config_file = File::create(&config_path).unwrap_or_else(|e| {
        panic!(
            "Unable to create config file '{}': {e}",
            config_path.display()
        )
    });

    warn!("Generating new config file: '{}'", config_path.display());

    let mut keybinding_map = indexmap::IndexMap::<KeyPattern, Action>::new();
    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: keysyms::KEY_Return.into(),
        },
        Action::Spawn(String::from("kitty")),
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super, KeyModifier::Shift]).into(),
            key: keysyms::KEY_q.into(),
        },
        Action::Quit,
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: keysyms::KEY_w.into(),
        },
        Action::Close,
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: keysyms::KEY_1.into(),
        },
        Action::Workspace(0),
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: keysyms::KEY_2.into(),
        },
        Action::Workspace(1),
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: keysyms::KEY_3.into(),
        },
        Action::Workspace(2),
    );

    let default_config = Config {
        workspaces: 3,
        keybindings: keybinding_map,
        gaps: default_gaps(),
        xkb: default_xkb(),
        autostart: default_autostart(),
        outputs: default_outputs(),
        borders: default_borders(),
    };
    let pretty = PrettyConfig::new().compact_arrays(true).depth_limit(2);

    debug!("Serializing default config");
    let ron = ron::ser::to_string_pretty(&default_config, pretty)
        .unwrap_or_else(|e| panic!("Unable to serialize config: {e}"));
    debug!("Writing config to file '{}'", config_path.display());
    config_file
        .write_all(ron.as_bytes())
        .unwrap_or_else(|e| panic!("Unable to write to config file: {e}"));
    config_path
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

fn default_outputs() -> IndexMap<String, OutputConfig> {
    IndexMap::new()
}

fn default_borders() -> Borders {
    Borders {
        thickness: 8,
        start_color: [0.880, 1.0, 1.0],
        end_color: Some([0.580, 0.921, 0.921]),
        radius: 8.0,
        gradient_angle: 0.0,
    }
}
