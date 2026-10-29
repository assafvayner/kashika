/// The neutral icon set shipped with Kashika, addressed by bare name (`icon="server"`).
pub const BUNDLED_ICONS: &[(&str, &str)] = &[
    ("server", include_str!("../icons/server.svg")),
    ("browser", include_str!("../icons/browser.svg")),
    ("queue", include_str!("../icons/queue.svg")),
    ("lock", include_str!("../icons/lock.svg")),
    ("cloud", include_str!("../icons/cloud.svg")),
    ("person", include_str!("../icons/person.svg")),
];

/// Search keywords per bundled icon, for `kashika icons --search`.
pub const ICON_KEYWORDS: &[(&str, &[&str])] = &[
    ("server", &["host", "machine", "vm", "compute", "backend"]),
    ("browser", &["web", "client", "frontend", "ui"]),
    ("queue", &["mq", "broker", "messages", "kafka", "stream"]),
    ("lock", &["security", "auth", "secret", "encryption"]),
    ("cloud", &["internet", "saas", "external"]),
    (
        "person",
        &["user", "users", "people", "human", "account", "actor"],
    ),
];

pub(crate) fn bundled(name: &str) -> Option<(&'static str, &'static str)> {
    BUNDLED_ICONS.iter().copied().find(|(n, _)| *n == name)
}
