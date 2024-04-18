use std::{env, fs::File, io::Read};
use xcursor::{parser, CursorTheme};

pub struct Xcursor {
    pixel_argb: Vec<u8>,
    size: u32,
    variant: String,
    theme: CursorTheme
}   

impl Xcursor {
    pub fn new() -> Self {
        let theme_str = match env::var("XCURSOR_THEME") {
            Ok(theme) => theme,
            Err(e) => {
                tracing::error!("Error fetching xcursor theme name: {}");
                tracing::warn!("Falling back to default xcursor theme");
                "default".to_owned();
            }
        };
        let theme = CursorTheme::load(theme_str.as_str());
        let variant = "default";
        // TODO: fall back to cursor bitmap included in magma's asset files
        let cursor_path = theme.load_icon(variant).unwrap();

        let mut cursor_data = Vec::new();
        let file = File::open(&cursor_path).unwrap();
        file.read_to_end(&mut cursor_data).unwrap();

        let images = parser::parse_xcursor(&cursor_data).unwrap();

        // placeholder. read env var for size
        // i wrote this on a phone, so i am super sorry :sob:
        let size = 24;

        let image = images.iter().rfind(|i| i.size == size).unwrap();

        let pixels_argb = image.pixels_argb;
        Self {
            pixels_argb,
            size,
            theme,
            variant: variant.to_owned()
        }
    }
}
