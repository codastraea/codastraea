use silkenweb::attribute::{AsAttribute, Attribute};
use strum::{Display, IntoStaticStr};

pub mod tab_group;

#[derive(Display, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum Edge {
    Top,
    Bottom,
    Start,
    End,
}

impl Attribute for Edge {
    type Text<'a> = &'a str;

    fn text(&self) -> Option<Self::Text<'_>> {
        Some(self.into())
    }
}

impl AsAttribute<Edge> for Edge {}
