//! What a Location naming this contract is configured with, and the name a
//! Validate step's `format` names it by: its declaration, which its
//! `ContractFactory` reads, built with the crate's `declaration` feature
//! alone and the code behind the default (ADR-0064, amendments 2026-10-22
//! and 2026-11-26), for the library the surfaces load to carry (open
//! problem 48).

use xcore::settings::{Applies, Fixed, Kind, Presence, Setting, Settings};

/// What a Location gives this contract (ADR-0064, amendment 2026-09-26).
pub const SETTINGS: &Settings = &Settings {
    technology: env!("CARGO_PKG_NAME"),
    settings: &[
        Setting {
            name: "reference",
            kind: Kind::Text,
            presence: Presence::Optional,
            meaning: "The pattern the text matches; left out, any UTF-8 text holds.",
            applies: Applies::Both,
        },
        Setting {
            name: "scope",
            kind: Kind::Choice {
                choices: &["whole", "lines"],
            },
            presence: Presence::Default(Fixed::Text("whole")),
            meaning: "Whether the whole text matches the pattern, or every non-empty line.",
            applies: Applies::Both,
        },
    ],
};

/// The technology's name, as a Validate step's `format` names it.
pub const NAME: &str = "regex";
