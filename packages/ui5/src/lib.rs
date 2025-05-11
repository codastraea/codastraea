use silkenweb::StrAttribute;
use strum::AsRefStr;

pub mod button;
pub mod icon;
pub mod menu;
pub mod tab;
pub mod tree;

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum Highlight {
    None,
    Positive,
    Critical,
    Negative,
    Information,
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum ItemType {
    Inactive,
    Active,
    Detail,
    Navigation,
}
