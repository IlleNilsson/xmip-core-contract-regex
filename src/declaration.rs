//! This contract's declaration: the contract guide,
//! `module/core/capability/contract/doc/adding-a-contract.md`.

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
