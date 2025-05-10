use silkenweb::StrAttribute;
use strum::AsRefStr;

pub mod icon;
pub mod tab;
pub mod tree;

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum TextState {
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
