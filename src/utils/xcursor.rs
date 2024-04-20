use std::{env, fs::File, io::Read};
use xcursor::{parser::{self, Image}, CursorTheme};

pub struct Xcursor {
    images: Vec<Image>,
    size: u32,
    variant: String,
    theme: CursorTheme
}   

impl Xcursor {
    pub fn new() -> Self {
        let theme_str = match env::var("XCURSOR_THEME") {
            Ok(theme) => theme,
            Err(e) => {
                tracing::error!("Error fetching xcursor theme name: {}", e);
                tracing::warn!("Falling back to default xcursor theme");
                "default".to_owned()
            }
        };
        let theme = CursorTheme::load(theme_str.as_str());
        let variant = "default";
        // TODO: fall back to cursor bitmap included in magma's asset files
        let cursor_path = theme.load_icon(variant).unwrap();

        let mut cursor_data = Vec::new();
        let mut file = File::open(&cursor_path).unwrap();
        file.read_to_end(&mut cursor_data).unwrap();

        let images = parser::parse_xcursor(&cursor_data).unwrap();

        let inputted_size = match env::var("XCURSOR_SIZE") {
            Ok(size) => {
                let size: u32 = size.parse().unwrap_or_else(|e| {
                    tracing::error!("Couldn't parse $XCURSOR_SIZE environment variable as numerical value: {}", e);
                    tracing::warn!("Falling back to 24 as the xcursor size");
                    24
                });
                size
            }
            Err(e) => {
                tracing::error!("Error reading $XCURSOR_SIZE environment variable: {}", e);
                tracing::warn!("Falling back to 24 as the xcursor size");
                24
            }
        };

        // Finds the nearest supported cursor size
        let size = images.iter().map(|i| i.size).min_by_key(|s| u32::abs_diff(inputted_size, *s)).unwrap();

        let images = images.iter().filter(|i| i.size == size).fold(Vec::new(), |mut accum, i| {
            accum.push(i.clone());
            accum
        });
        
        Self {
            images,
            size,
            variant: variant.to_owned(),
            theme
        }
    }
}
