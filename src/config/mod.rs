use std::{collections::HashMap, fs::File, fs::OpenOptions, io, io::Write, path::PathBuf};

use self::types::{deserialize_KeyModifiers, deserialize_Keysym, XkbConfig};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use smithay::utils::{Physical, Size};

mod types;
#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Clone, Serialize)]
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
        let file_path = xdg
            .place_data_file("magamawm/config.ron")
            .expect("Failed to get file path");
        let mut file = match File::create(file_path.clone()) {
            Ok(file) => file,
            Err(err) => {
                println!("Failed to create file: {}", err);
                panic!("Couldnt create config file")
            }
        };

        let mut keybinding_map = std::collections::HashMap::<KeyPattern, Action>::new();
        keybinding_map.insert(KeyPattern{
            modifiers: KeyModifiers{
                ctrl: true,
                alt: false,
                shift: false,
                logo: false
            },
            key: 0xff0d
        }, Action::Spawn(String::from("alacritty")));

        keybinding_map.insert(KeyPattern{
            modifiers: KeyModifiers{
                ctrl: true,
                alt: false,
                shift: false,
                logo: false
            },
            key: 0x0071
        }, Action::Quit);

        keybinding_map.insert(KeyPattern{
            modifiers: KeyModifiers{
                ctrl: true,
                alt: false,
                shift: false,
                logo: false
            },
            key: 0x0077 
        }, Action::Close);

        keybinding_map.insert(KeyPattern{
            modifiers: KeyModifiers{
                ctrl: true,
                alt: false,
                shift: false,
                logo: false
            },
            key: 0xffb1
        }, Action::Workspace(1));

        keybinding_map.insert(KeyPattern{
            modifiers: KeyModifiers{
                ctrl: true,
                alt: false,
                shift: false,
                logo: false
            },
            key: 0xffb2
        }, Action::Workspace(2));

        keybinding_map.insert(KeyPattern{
            modifiers: KeyModifiers{
                ctrl: true,
                alt: false,
                shift: false,
                logo: false
            },
            key: 0xffb3
        }, Action::Workspace(3));

        
        let default_config = Config { workspaces: 8, keybindings: keybinding_map, gaps: default_gaps(), xkb: default_xkb(), autostart:default_autostart(), outputs: default_outputs() };
        let test_thing = ron::ser::to_string(&keybinding_map.clone()).unwrap();
        file.write_all(
            b"",
        )
        .expect("ERROR: Couldnt write to file");
        return file_path;
    }
    if input.trim() == "n" {
        println!("OK, exitting...");
        panic!("No config file found");
    } else {
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
    return ron::de::from_reader(
        OpenOptions::new()
            .read(true)
            .open(generate_config())
            .unwrap(),
    )
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
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
    pub modifiers: KeyModifiers,
    /// The actual key, that was pressed
    #[serde(deserialize_with = "deserialize_Keysym")]
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
    VTSwitch(i32),
    Spawn(String),
}
