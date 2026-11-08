//! What the bot calls a club's ticket channels. See the README's "The ticket name".

/// Longest prefix, and longest override, once cleaned up.
const MAX_PREFIX: usize = 32;
const MAX_OVERRIDE: usize = 80;

/// Longest text a leader may send, before it is cleaned up.
const MAX_INPUT: usize = MAX_OVERRIDE;

/// Longest slice of the trainer's name in a channel name.
const MAX_TRAINER: usize = 40;

/// Longest channel name we make; Discord takes 100.
const MAX_NAME: usize = 90;

/// The dash and four digits at the end of every name.
const NUMBER_LEN: usize = 5;

/// How a club's ticket channels are named.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TicketName {
    /// `<club>-<trainer>-<number>`.
    #[default]
    Default,
    /// `<prefix>-<trainer>-<number>`.
    Prefix(String),
    /// `<override>-<number>`.
    Override(String),
}

/// Why a leader's text can't name tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    /// Nothing is left after trimming.
    Empty,
    /// Too long.
    TooLong,
    /// Anything but letters, digits and single dashes.
    BadCharacters,
}

impl NameError {
    /// A message for the leader.
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "the ticket name can't be empty",
            Self::TooLong => "the ticket name is too long",
            Self::BadCharacters => {
                "the ticket name can only have letters, numbers and single dashes"
            }
        }
    }
}

impl TicketName {
    /// A prefix from the leader's text.
    pub fn with_prefix(text: &str) -> Result<Self, NameError> {
        clean(text, MAX_PREFIX).map(Self::Prefix)
    }

    /// An override from the leader's text.
    pub fn with_override(text: &str) -> Result<Self, NameError> {
        clean(text, MAX_OVERRIDE).map(Self::Override)
    }

    /// What's stored for a club, checked again so a bad value never names a channel. An override
    /// wins over a prefix, and anything invalid is the default.
    pub(crate) fn from_stored(prefix: Option<&str>, name: Option<&str>) -> Self {
        let name = name.map(Self::with_override);
        let prefix = prefix.map(Self::with_prefix);
        name.or(prefix).and_then(Result::ok).unwrap_or_default()
    }

    /// The fields to store, as the update script takes them: the other one is emptied.
    pub(crate) fn fields(&self) -> [(&'static str, String); 2] {
        match self {
            Self::Default => [
                ("ticket_prefix", String::new()),
                ("ticket_name", String::new()),
            ],
            Self::Prefix(prefix) => [
                ("ticket_prefix", prefix.clone()),
                ("ticket_name", String::new()),
            ],
            Self::Override(name) => [
                ("ticket_prefix", String::new()),
                ("ticket_name", name.clone()),
            ],
        }
    }

    /// The channel's name for an applicant to a club, ending in `number`'s last four digits.
    pub(crate) fn channel_name(&self, club_name: &str, trainer_name: &str, number: u16) -> String {
        let number = format!("{:04}", number % 10_000);
        let head = match self {
            Self::Override(name) => return format!("{name}-{number}"),
            Self::Prefix(prefix) => prefix.clone(),
            Self::Default => slug(club_name),
        };
        let mut trainer = slug(trainer_name);
        if trainer.is_empty() {
            trainer = "applicant".to_owned();
        }
        let trainer = cut(trainer, MAX_TRAINER);
        // The club or prefix gives way first, so the trainer and number always fit.
        let head = cut(head, MAX_NAME - NUMBER_LEN - 1 - trainer.len());
        if head.is_empty() {
            format!("{trainer}-{number}")
        } else {
            format!("{head}-{trainer}-{number}")
        }
    }
}

/// A random number to tell tickets apart, 0 to 9,999.
pub(crate) fn random_number() -> u16 {
    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).expect("the operating system's random number generator works");
    (u32::from_le_bytes(bytes) % 10_000) as u16
}

/// `text` as a channel name part, or why it can't be one. Spaces and underscores become dashes;
/// nothing else is changed, so anything outside letters and digits is refused, not dropped.
fn clean(text: &str, max: usize) -> Result<String, NameError> {
    if text.len() > MAX_INPUT {
        return Err(NameError::TooLong);
    }
    let name: String = text
        .trim()
        .chars()
        .map(|c| match c {
            ' ' | '_' => '-',
            c => c.to_ascii_lowercase(),
        })
        .collect();
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    let plain = name.split('-').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    });
    if !plain {
        return Err(NameError::BadCharacters);
    }
    if name.len() > max {
        return Err(NameError::TooLong);
    }
    Ok(name)
}

/// Lowercase letters and digits of `text` joined by dashes, which may leave nothing. For names
/// from elsewhere, which are cleaned up rather than refused.
fn slug(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_owned()
}

/// `text`, which is ASCII, cut to `max` bytes without a dash at the end.
fn cut(mut text: String, max: usize) -> String {
    text.truncate(max);
    text.truncate(text.trim_end_matches('-').len());
    text
}

#[cfg(test)]
mod tests;
