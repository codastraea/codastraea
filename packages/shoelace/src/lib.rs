use silkenweb::{StrAttribute, Value};
use strum::{AsRefStr, Display};

pub mod tab;
pub mod tree;

#[derive(Display, AsRefStr, StrAttribute, Value)]
#[strum(serialize_all = "kebab-case")]
pub enum Edge {
    Top,
    Bottom,
    Start,
    End,
}
