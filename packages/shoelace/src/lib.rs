use silkenweb::{StrAttribute, Value};
use strum::{AsRefStr, Display};

pub mod button;
pub mod dropdown;
pub mod icon;
pub mod menu;
pub mod tab;
pub mod tree;

#[derive(Copy, Clone, Eq, PartialEq, Display, AsRefStr, StrAttribute, Value)]
#[strum(serialize_all = "kebab-case")]
pub enum Edge {
    Top,
    Bottom,
    Start,
    End,
}

#[derive(Copy, Clone, Eq, PartialEq, Display, AsRefStr, StrAttribute, Value)]
#[strum(serialize_all = "kebab-case")]
pub enum Size {
    Small,
    Medium,
    Large,
}
