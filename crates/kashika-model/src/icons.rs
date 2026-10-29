include!(concat!(env!("OUT_DIR"), "/icons.rs"));

pub(crate) fn bundled(name: &str) -> Option<(&'static str, &'static str)> {
    BUNDLED_ICONS.iter().copied().find(|(n, _)| *n == name)
}
