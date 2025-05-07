use derive_more::Into;
use serpent_automation_executor::syntax_tree::SrcSpan;
use silkenweb::{
    node::Node,
    prelude::{
        html::div, Element
    },
    Value,
};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::css;

#[derive(Into, Value)]
pub struct SourceView(Node);

impl SourceView {
    pub fn new(editor: &Editor) -> Self {
        let codemirror_container = div().class(css::FULL_HEIGHT);
        codemirror_container
            .handle()
            .dom_element()
            .append_child(&editor.0.dom())
            .unwrap();

        Self(codemirror_container.into())
    }
}

#[derive(Clone)]
pub struct Editor(EditorView);

impl Editor {
    pub fn new(code: &str) -> Self {
        Self(codemirror_new(code))
    }

    pub fn set_selection(&self, span: SrcSpan) {
        let start_pos = self.0.state().doc().line(span.line()).from() + span.column() - 1;
        set_selection(&self.0, start_pos, start_pos + span.len());
    }
}

#[wasm_bindgen(raw_module = "/serpent-automation-js-bundle.esm.js")]
extern "C" {
    // TODO: Can any of these throw exceptions?
    #[derive(Clone)]
    type EditorView;

    #[wasm_bindgen]
    fn codemirror_new(doc: &str) -> EditorView;

    #[wasm_bindgen(method, getter)]
    fn dom(this: &EditorView) -> web_sys::HtmlElement;

    #[wasm_bindgen(method, getter)]
    fn state(this: &EditorView) -> EditorState;

    type EditorState;

    #[wasm_bindgen(method, getter)]
    fn doc(this: &EditorState) -> Text;

    type Text;

    #[wasm_bindgen(method)]
    fn line(this: &Text, line_num: usize) -> Line;

    type Line;

    #[wasm_bindgen(method, getter)]
    fn from(this: &Line) -> usize;

    #[wasm_bindgen]
    fn set_selection(editor: &EditorView, from: usize, to: usize) -> usize;
}
