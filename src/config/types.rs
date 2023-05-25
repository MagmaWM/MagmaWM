use colors_transform::{Color, Rgb};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use smithay::input::keyboard::{
    keysyms as KeySyms, xkb, Keysym, ModifiersState, XkbConfig as WlXkbConfig,
};
use tracing::warn;

use super::{KeyModifier, KeyModifiers};

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct KeyModifiersDef(pub Vec<KeyModifier>);

impl From<KeyModifiersDef> for KeyModifiers {
    fn from(src: KeyModifiersDef) -> Self {
        src.0.into_iter().fold(
            KeyModifiers {
                ctrl: false,
                alt: false,
                shift: false,
                logo: false,
            },
            |mut modis, modi: KeyModifier| {
                modis += modi;
                modis
            },
        )
    }
}

#[allow(non_snake_case)]
pub fn deserialize_KeyModifiers<'de, D>(deserializer: D) -> Result<KeyModifiers, D::Error>
where
    D: serde::Deserializer<'de>,
{
    KeyModifiersDef::deserialize(deserializer).map(Into::into)
}

#[allow(non_snake_case)]
pub fn serialize_KeyModifiers<S>(
    key_modifiers: &KeyModifiers,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut modifiers = vec![];
    if key_modifiers.logo {
        modifiers.push(KeyModifier::Super);
    }
    if key_modifiers.shift {
        modifiers.push(KeyModifier::Shift);
    }
    if key_modifiers.ctrl {
        modifiers.push(KeyModifier::Ctrl);
    }
    if key_modifiers.alt {
        modifiers.push(KeyModifier::Alt);
    }
    let mut seq = serializer.serialize_seq(Some(modifiers.len()))?;
    for e in modifiers {
        seq.serialize_element(&e)?;
    }
    seq.end()
}

#[allow(non_snake_case)]
pub fn deserialize_Keysym<'de, D>(deserializer: D) -> Result<Keysym, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, Unexpected};

    let name = String::deserialize(deserializer)?;
    //let name = format!("KEY_{}", code);
    match xkb::keysym_from_name(&name, xkb::KEYSYM_NO_FLAGS) {
        KeySyms::KEY_NoSymbol => match xkb::keysym_from_name(&name, xkb::KEYSYM_CASE_INSENSITIVE) {
            KeySyms::KEY_NoSymbol => Err(<D::Error as Error>::invalid_value(
                Unexpected::Str(&name),
                &"One of the keysym names of xkbcommon.h without the 'KEY_' prefix",
            )),
            x => {
                warn!(
                    "Key-Binding '{}' only matched case insensitive for {:?}",
                    name,
                    xkb::keysym_get_name(x)
                );
                Ok(x)
            }
        },
        x => Ok(x),
    }
}

#[allow(non_snake_case)]
pub fn serialize_Keysym<S>(keysym: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&xkb::keysym_get_name(*keysym))
}

impl std::ops::AddAssign<KeyModifier> for KeyModifiers {
    fn add_assign(&mut self, rhs: KeyModifier) {
        match rhs {
            KeyModifier::Ctrl => self.ctrl = true,
            KeyModifier::Alt => self.alt = true,
            KeyModifier::Shift => self.shift = true,
            KeyModifier::Super => self.logo = true,
        };
    }
}

impl PartialEq<ModifiersState> for KeyModifiers {
    fn eq(&self, other: &ModifiersState) -> bool {
        self.ctrl == other.ctrl
            && self.alt == other.alt
            && self.shift == other.shift
            && self.logo == other.logo
    }
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct XkbConfig {
    pub rules: String,
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: Option<String>,
}

impl<'a> From<&'a XkbConfig> for WlXkbConfig<'a> {
    fn from(val: &'a XkbConfig) -> Self {
        WlXkbConfig {
            rules: &val.rules,
            model: &val.model,
            layout: &val.layout,
            variant: &val.variant,
            options: val.options.clone(),
        }
    }
}

#[allow(non_snake_case)]
pub fn deserialize_StartColour<'de, D>(deserializer: D) -> Result<[f32; 3], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let hex = String::deserialize(deserializer)?;
    let rgb = Rgb::from_hex_str(&hex)
        .map_err(|err| <D::Error as Error>::custom(err.message))?
        .as_tuple();
    Ok([rgb.0 / 255.0, rgb.1 / 255.0, rgb.2 / 255.0])
}

#[allow(non_snake_case)]
pub fn serialize_StartColour<S>(rgb: &[f32; 3], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &Rgb::from(rgb[0] * 255.0, rgb[1] * 255.0, rgb[2] * 255.0).to_css_hex_string(),
    )
}

#[allow(non_snake_case)]
pub fn deserialize_EndColour<'de, D>(deserializer: D) -> Result<Option<[f32; 3]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    if let Some(hex) = Option::<String>::deserialize(deserializer)? {
        let rgb = Rgb::from_hex_str(&hex)
            .map_err(|err| <D::Error as Error>::custom(err.message))?
            .as_tuple();
        Ok(Some([rgb.0 / 255.0, rgb.1 / 255.0, rgb.2 / 255.0]))
    } else {
        Ok(None)
    }
}

#[allow(non_snake_case)]
pub fn serialize_EndColour<S>(rgb: &Option<[f32; 3]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(rgb) = rgb {
        serializer.serialize_some(
            &Rgb::from(rgb[0] * 255.0, rgb[1] * 255.0, rgb[2] * 255.0).to_css_hex_string(),
        )
    } else {
        serializer.serialize_none()
    }
}
