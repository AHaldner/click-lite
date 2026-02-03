mod chat_area;
mod header;
mod rich_text;
mod sidebar;

pub use chat_area::render_chat_area;
pub use header::render_header;
pub use rich_text::RichText;
pub use sidebar::render_sidebar;

pub fn stable_u64_hash(value: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
