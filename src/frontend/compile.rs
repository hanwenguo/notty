use typst::{diag::{SourceResult, Warned}, html::HtmlDocument, WorldExt};

use crate::frontend::copied::world::SystemWorld;

pub fn compile(world: &mut SystemWorld) -> Warned<SourceResult<HtmlDocument>> {
    typst::compile::<HtmlDocument>(world)
}