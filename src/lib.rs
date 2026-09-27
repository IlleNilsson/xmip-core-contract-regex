#![forbid(unsafe_code)]

//! The regular-expression content contract — a technology of `xmip-core-contract`.
//!
//! Two claims, decided 2026-09-07: **well-formedness is a given** — here, that
//! the Stream is UTF-8 text — and **conformance is a given once a contract is
//! named**: a Receive or Send Location that refers to this contract with a
//! pattern bound has every Stream held to that pattern.
//!
//! A pattern binds in one of two scopes. **Whole**: the entire text must match
//! the pattern, anchored at both ends. **Lines**: every non-empty line must
//! match, anchored at both ends, and each departure names its line — the
//! shape of a flat file whose every record has one form. A Location says which
//! in its `scope` setting; the C boundary's `load` carries one string, so
//! there `lines:<pattern>` is per line and anything else is whole. The syntax is the `regex` crate's: finite automata, no backreferences,
//! linear in the input, so an operator's pattern cannot stall a Location.

use contract::{
    Contract, ContractDescriptor, ContractError, ContractFactory, ContractId, ValidationIssue,
    ValidationResult,
};
use regex::Regex;
use stream::Stream;
use xcore::settings::{Applies, Fixed, Kind, Presence, Read, Setting, Settings};

/// How a bound pattern is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The whole text matches, start to end.
    Whole,
    /// Every non-empty line matches, start to end.
    Lines,
}

/// The regular-expression contract, bare or bound to a pattern.
pub struct RegexContract {
    descriptor: ContractDescriptor,
    bound: Option<(Regex, Scope)>,
}

impl RegexContract {
    /// UTF-8 text only.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: descriptor("regex"),
            bound: None,
        }
    }

    /// UTF-8 text that matches `pattern` in `scope`.
    ///
    /// # Errors
    /// The pattern must be valid `regex` syntax.
    pub fn with_pattern(pattern: &str, scope: Scope) -> Result<Self, ContractError> {
        let anchored = format!("^(?:{pattern})$");
        let compiled = Regex::new(&anchored).map_err(|error| ContractError {
            message: format!("pattern refused: {error}"),
        })?;
        let id = match scope {
            Scope::Whole => format!("regex:{pattern}"),
            Scope::Lines => format!("regex:lines:{pattern}"),
        };
        Ok(Self {
            descriptor: descriptor(&id),
            bound: Some((compiled, scope)),
        })
    }

    /// Whether a pattern is bound.
    #[must_use]
    pub fn is_bound(&self) -> bool {
        self.bound.is_some()
    }
}

impl Default for RegexContract {
    fn default() -> Self {
        Self::new()
    }
}

fn descriptor(id: &str) -> ContractDescriptor {
    ContractDescriptor {
        id: ContractId(id.to_string()),
        version: "1".to_string(),
        representation: "text/plain".to_string(),
    }
}

impl Contract for RegexContract {
    fn descriptor(&self) -> &ContractDescriptor {
        &self.descriptor
    }

    fn identify(&self, stream: &Stream) -> Result<bool, ContractError> {
        // Text is the claim: any Stream that decodes is a candidate. A bound
        // pattern does not narrow identify — that is validate's job, and an
        // operator wants "does not match" reported, not silently unclaimed.
        Ok(std::str::from_utf8(stream.bytes()).is_ok())
    }

    fn validate(&self, stream: &Stream) -> Result<ValidationResult, ContractError> {
        let text = match std::str::from_utf8(stream.bytes()) {
            Ok(text) => text,
            Err(error) => {
                return Ok(ValidationResult::of(vec![ValidationIssue::at(
                    "malformed",
                    &format!("not UTF-8 text: {error}"),
                    &format!("byte {}", error.valid_up_to()),
                )]));
            }
        };
        let Some((pattern, scope)) = &self.bound else {
            return Ok(ValidationResult::of(Vec::new()));
        };
        let issues = match scope {
            Scope::Whole => {
                if pattern.is_match(text.trim_end_matches(['\r', '\n'])) {
                    Vec::new()
                } else {
                    vec![ValidationIssue::new(
                        "pattern",
                        "the text does not match the pattern",
                        None,
                    )]
                }
            }
            Scope::Lines => text
                .lines()
                .enumerate()
                .filter(|(_, line)| !line.is_empty() && !pattern.is_match(line))
                .map(|(offset, _)| {
                    ValidationIssue::new(
                        "pattern",
                        "the line does not match the pattern",
                        Some(format!("line {}", offset + 1)),
                    )
                })
                .collect(),
        };
        Ok(ValidationResult::of(issues))
    }
}

/// Loads the contract a Location names: an empty reference is the bare
/// contract, anything else the pattern, bound in the Location's `scope`.
/// The C boundary's `load` carries one string, so there `lines:<pattern>`
/// binds per line and anything else binds whole.
pub struct RegexContractFactory;

impl ContractFactory for RegexContractFactory {
    fn technology(&self) -> &'static str {
        "regex"
    }

    fn settings(&self) -> &'static Settings {
        SETTINGS
    }

    fn configured(&self, settings: &Read) -> Result<Box<dyn Contract>, ContractError> {
        let pattern = settings.optional_text("reference").unwrap_or("");
        if pattern.trim().is_empty() {
            return Ok(Box::new(RegexContract::new()));
        }
        let scope = match settings.text("scope") {
            "lines" => Scope::Lines,
            _ => Scope::Whole,
        };
        Ok(Box::new(RegexContract::with_pattern(pattern, scope)?))
    }

    fn load(&self, reference: &str) -> Result<Box<dyn Contract>, ContractError> {
        if reference.trim().is_empty() {
            return Ok(Box::new(RegexContract::new()));
        }
        let contract = match reference.strip_prefix("lines:") {
            Some(pattern) => RegexContract::with_pattern(pattern, Scope::Lines)?,
            None => RegexContract::with_pattern(reference, Scope::Whole)?,
        };
        Ok(Box::new(contract))
    }
}

/// What a Location gives this contract (ADR-0064, amendment 2026-09-26).
const SETTINGS: &Settings = &Settings {
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

#[cfg(test)]
mod tests {
    use super::*;
    use xcore::StreamId;

    fn stream(bytes: &[u8]) -> Stream {
        Stream::new(
            StreamId::new(1),
            bytes.to_vec(),
            Some("text/plain".to_string()),
        )
    }

    #[test]
    fn bare_contract_holds_any_text_and_refuses_bytes() {
        let bare = RegexContract::new();
        assert!(
            bare.validate(&stream(b"anything"))
                .expect("validates")
                .valid
        );
        let broken = bare.validate(&stream(&[0xff, 0xfe])).expect("validates");
        assert_eq!(broken.issues[0].code, "malformed");
        assert_eq!(broken.issues[0].path.as_deref(), Some("byte 0"));
    }

    #[test]
    fn whole_scope_is_anchored_at_both_ends() {
        let bound = RegexContract::with_pattern(r"[A-Z]{2}\d{4}", Scope::Whole).expect("pattern");
        assert!(
            bound
                .validate(&stream(b"SE1234\n"))
                .expect("validates")
                .valid
        );
        let wrong = bound.validate(&stream(b"xSE1234")).expect("validates");
        assert_eq!(wrong.issues[0].code, "pattern");
        assert_eq!(bound.descriptor().id.0, r"regex:[A-Z]{2}\d{4}");
    }

    #[test]
    fn lines_scope_names_each_line_that_departs() {
        let bound = RegexContract::with_pattern(r"\d+;\w+", Scope::Lines).expect("pattern");
        let held = bound
            .validate(&stream(b"1;a\n\n2;b\nthree;c\n4 d\n"))
            .expect("validates");
        assert!(!held.valid);
        let lines: Vec<&str> = held
            .issues
            .iter()
            .filter_map(|i| i.path.as_deref())
            .collect();
        assert_eq!(lines, ["line 4", "line 5"]);
    }

    #[test]
    fn a_bad_pattern_is_refused_when_bound_not_when_validating() {
        let error = RegexContract::with_pattern("(unclosed", Scope::Whole)
            .err()
            .expect("refused");
        assert!(error.message.starts_with("pattern refused"));
    }

    #[test]
    fn identify_is_the_text_claim_alone() {
        let bound = RegexContract::with_pattern("x", Scope::Whole).expect("pattern");
        assert!(bound.identify(&stream(b"not x")).expect("identifies"));
        assert!(!bound.identify(&stream(&[0xff])).expect("identifies"));
    }

    #[test]
    fn the_factory_reads_the_scope_off_the_reference() {
        let factory = RegexContractFactory;
        assert_eq!(factory.technology(), "regex");
        assert_eq!(factory.load("").expect("bare").descriptor().id.0, "regex");
        assert_eq!(
            factory.load("a+").expect("whole").descriptor().id.0,
            "regex:a+"
        );
        let lines = factory.load("lines:a+").expect("lines");
        assert_eq!(lines.descriptor().id.0, "regex:lines:a+");
        assert!(
            lines
                .validate(&stream(b"a\naa\n"))
                .expect("validates")
                .valid
        );
        assert!(factory.load("(").is_err());
    }

    #[test]
    fn regex_declares_its_settings_and_reads_through_them() {
        use xcore::settings::Given;
        assert!(SETTINGS.problems().is_empty(), "{:?}", SETTINGS.problems());
        let given = |name: &str, value: &str| (name.to_string(), Given::Text(value.to_string()));
        let factory = RegexContractFactory;
        let lines = factory
            .open(
                Applies::Receive,
                &[given("reference", "[A-Z]+"), given("scope", "lines")],
            )
            .expect("bound per line");
        assert_eq!(lines.descriptor().id.0, "regex:lines:[A-Z]+");
        let text = stream(b"AB\ncd\n");
        assert_eq!(lines.validate(&text).expect("validates").issues.len(), 1);
        let whole = factory
            .open(Applies::Send, &[given("reference", "[A-Z]+")])
            .expect("bound whole by default");
        assert_eq!(whole.descriptor().id.0, "regex:[A-Z]+");
        assert!(factory.open(Applies::Both, &[]).is_ok(), "bare");
        let refused = factory
            .open(Applies::Receive, &[given("scope", "rows")])
            .err()
            .expect("a scope that is not a choice is refused");
        assert!(refused.message.contains("scope"), "{}", refused.message);
    }
}
