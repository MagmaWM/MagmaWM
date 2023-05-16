use std::{collections::HashMap, fs::OpenOptions, fs::File, path::PathBuf, io::Write, io};

use self::types::{deserialize_KeyModifiers, deserialize_Keysym, XkbConfig};
use serde::Deserialize;
use smithay::utils::{Physical, Size};

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

    #[serde(default = "default_outputs")]
    pub outputs: HashMap<String, OutputConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig((i32, i32), Option<u32>);

impl OutputConfig {
    pub fn mode_size(&self) -> Size<i32, Physical> {
        self.0.into()
    }

    pub fn mode_refresh(&self) -> u32 {
        self.1.unwrap_or(60_000)
    }
}

pub fn generate_config() -> PathBuf {
    println!("Would you like to generate a config file? [y/N]");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    if input.trim() == "y" {
        println!("OK, generating config...");
        let xdg = xdg::BaseDirectories::new().expect("Couldnt get xdg basedirs");
        let file_path = xdg.place_data_file("magamawm/config.ron").expect("Failed to get file path");
        let mut file = match File::create(file_path.clone()) {
            Ok(file) => file,
            Err(err) => {
                println!("Failed to create file: {}", err);
                panic!("Couldnt create config file")
            }
        };
        file.write_all(b"(
            workspaces: 3,
            keybindings: {
                (modifiers: [Ctrl], key: \"Return\"): Spawn(\"alacritty\"),
                (modifiers: [Ctrl], key: \"q\"): Quit,
                (modifiers: [Ctrl], key: \"w\"): Close,
                (modifiers: [Ctrl], key: \"1\"): Workspace(0),
                (modifiers: [Ctrl], key: \"2\"): Workspace(1),
                (modifiers: [Ctrl], key: \"3\"): Workspace(2),
            },
        )").expect("ERROR: Couldnt write to file");
        return file_path;
    }
    if input.trim() == "n" {
        println!("OK, exitting...");
        panic!("No config file found");
    }
    else {
        println!("ERROR: Unknown input, try again");
        panic!();
    }
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
    dbg!("No config file found in default locations, prompting generation");
    return ron::de::from_reader(OpenOptions::new().read(true).open(generate_config()).unwrap())
        .expect("Malformed config file");
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

fn default_outputs() -> HashMap<String, OutputConfig> {
    HashMap::new()
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
