use std::{fs::File, fs::OpenOptions, io::Write, path::PathBuf};

use crate::config::types::KeyModifiersDef;

use self::types::{
    deserialize_KeyModifiers, deserialize_Keysym, serialize_KeyModifiers, serialize_Keysym,
    XkbConfig,
};
use indexmap::IndexMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use smithay::{
    input::keyboard::xkb,
    utils::{Physical, Size},
};
use tracing::{info, warn};

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

    #[serde(default = "default_tile_ratio_update_interval")]
    pub tile_ratio_update_interval: f32,
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

pub fn generate_config() -> PathBuf {
    warn!("No config file found, generating one");
    let xdg = xdg::BaseDirectories::new().expect("Couldnt get xdg basedirs");
    let file_path = xdg
        .place_config_file("magmawm/config.ron")
        .expect("Failed to get file path");
    let mut file = match File::create(file_path.clone()) {
        Ok(file) => file,
        Err(err) => {
            println!("Failed to create file: {}", err);
            panic!("Couldnt create config file")
        }
    };

    let mut keybinding_map = indexmap::IndexMap::<KeyPattern, Action>::new();
    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: xkb::KEY_Return,
        },
        Action::Spawn(String::from("kitty")),
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super, KeyModifier::Shift]).into(),
            key: xkb::KEY_q,
        },
        Action::Quit,
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: xkb::KEY_w,
        },
        Action::Close,
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: xkb::KEY_a,
        },
        Action::DecreaseTileRatio,
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: xkb::KEY_d,
        },
        Action::IncreaseTileRatio,
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: xkb::KEY_1,
        },
        Action::Workspace(0),
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: xkb::KEY_2,
        },
        Action::Workspace(1),
    );

    keybinding_map.insert(
        KeyPattern {
            modifiers: KeyModifiersDef(vec![KeyModifier::Super]).into(),
            key: xkb::KEY_3,
        },
        Action::Workspace(2),
    );

    // order hashmap using indexmap

    let default_config = Config {
        workspaces: 3,
        keybindings: keybinding_map,
        gaps: default_gaps(),
        xkb: default_xkb(),
        autostart: default_autostart(),
        outputs: default_outputs(),
        tile_ratio_update_interval: default_tile_ratio_update_interval(),
    };
    let pretty = PrettyConfig::new().compact_arrays(true).depth_limit(2);
    let ron = ron::ser::to_string_pretty(&default_config, pretty).unwrap();
    file.write_all(ron.as_bytes())
        .expect("ERROR: Couldnt write to file");
    file_path
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
        info!("Trying config location: {}", path.display());
        if path.exists() {
            info!("Using config at {}", path.display());
            return ron::de::from_reader(OpenOptions::new().read(true).open(path).unwrap())
                .expect("Malformed config file");
        }
    }
    info!("No config file found in default locations, prompting generation");
    return ron::de::from_reader(
        OpenOptions::new()
            .read(true)
            .open(generate_config())
            .unwrap(),
    )
    .unwrap();
}

fn default_gaps() -> (i32, i32) {
    (5, 5)
}

fn default_tile_ratio_update_interval() -> f32 {
    0.2f32
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

/// Describtion of a key combination that might be
/// handled by the compositor.
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
    pub key: u32,
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
    IncreaseTileRatio,
    DecreaseTileRatio,
    VTSwitch(i32),
    Spawn(String),
}
