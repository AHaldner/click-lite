mod chat_area;
mod header;
mod sidebar;

pub use chat_area::render_chat_area;
pub use header::render_header;
pub use sidebar::render_sidebar;

pub mod colors {
    use gpui::{Rgba, rgb};

    pub fn sidebar_bg() -> Rgba {
        rgb(0x3f0e40).into()
    }
    pub fn sidebar_border() -> Rgba {
        rgb(0x2b0a2c).into()
    }
    pub fn main_bg() -> Rgba {
        rgb(0x1a1d21).into()
    }
    pub fn header_bg() -> Rgba {
        rgb(0x222529).into()
    }
    pub fn divider() -> Rgba {
        rgb(0x2c2f33).into()
    }
    pub fn text_primary() -> Rgba {
        rgb(0xf2f2f2).into()
    }
    pub fn text_secondary() -> Rgba {
        rgb(0x8e9297).into()
    }
    pub fn text_muted() -> Rgba {
        rgb(0x6e7177).into()
    }
    pub fn accent() -> Rgba {
        rgb(0x1164a3).into()
    }
    pub fn accent_hover() -> Rgba {
        rgb(0x0d5a8c).into()
    }
    pub fn card_bg() -> Rgba {
        rgb(0x2a2d31).into()
    }
    pub fn sidebar_text() -> Rgba {
        rgb(0xd1cbd4).into()
    }
    pub fn sidebar_icon() -> Rgba {
        rgb(0x9d9da0).into()
    }
}

pub fn stable_u64_hash(value: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
