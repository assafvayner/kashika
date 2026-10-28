/// The neutral icon set shipped with Kashika, addressed by bare name (`icon="server"`).
pub const BUNDLED_ICONS: &[(&str, &str)] = &[
    ("server", include_str!("../icons/server.svg")),
    ("browser", include_str!("../icons/browser.svg")),
    ("queue", include_str!("../icons/queue.svg")),
    ("lock", include_str!("../icons/lock.svg")),
    ("cloud", include_str!("../icons/cloud.svg")),
    ("person", include_str!("../icons/person.svg")),
];

pub(crate) fn bundled(name: &str) -> Option<(&'static str, &'static str)> {
    BUNDLED_ICONS.iter().copied().find(|(n, _)| *n == name)
}
