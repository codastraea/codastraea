use std::{cell::RefCell, collections::HashMap, rc::Rc};

use futures_signals::signal::{Mutable, Signal};
use gloo_console::log;
use serpent_automation_executor::{
    library::Library,
    run::{CallStack, RunState, ThreadCallStates},
    CODE,
};
use serpent_automation_frontend::ReceiveCallStates;
use silkenweb::{
    elements::html::div,
    node::{element::ElementBuilder, Node},
    prelude::ParentBuilder,
};
use silkenweb_bootstrap::{
    row,
    utility::{Align, Overflow, SetDisplay, SetOverflow, SetSpacing, Size::Size3},
};
use thread_view::ThreadView;
use wasm_bindgen::prelude::wasm_bindgen;

mod animation;
mod splitter;
mod call_tree_view;
mod thread_view;
mod css {
    silkenweb::css_classes!(visibility: pub, path: "serpent-automation.css");
}

#[wasm_bindgen(raw_module = "/codemirror.esm.js")]
extern "C" {
    // TODO: Can any of these throw exceptions?
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

pub fn app(library: &Rc<Library>, view_call_states: &ViewCallStates) -> impl Into<Node> {
    let main_id = library.main_id().unwrap();

    let codemirror_container = div().overflow(Overflow::Auto);
    let editor = codemirror_new(CODE);

    let pos = editor.state().doc().line(2).from();
    set_selection(&editor, pos + 2, pos + 4);

    codemirror_container
        .handle()
        .dom_element()
        .append_child(&editor.dom())
        .unwrap();

    row()
        .margin(Some(Size3))
        .align_items(Align::Start)
        .child(ThreadView::new(main_id, library, view_call_states))
        .overflow(Overflow::Auto)
}

#[derive(Clone, Default)]
pub struct ViewCallStates(Rc<RefCell<ViewCallStatesData>>);

impl ViewCallStates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_state(&self, call_stack: &CallStack) -> impl Signal<Item = RunState> {
        let mut data = self.0.borrow_mut();

        if let Some(existing) = data.view_call_states.get(call_stack) {
            existing
        } else {
            let new = Mutable::new(data.call_states.run_state(call_stack));
            data.view_call_states
                .entry(call_stack.clone())
                .or_insert(new)
        }
        .signal()
    }
}

impl ReceiveCallStates for ViewCallStates {
    fn set_call_states(&self, thread_state: ThreadCallStates) {
        let mut data = self.0.borrow_mut();

        for (call_stack, run_state) in &data.view_call_states {
            log!(format!("call stack {:?}", call_stack));
            run_state.set_neq(thread_state.run_state(call_stack));
        }

        data.call_states = thread_state;
    }
}

#[derive(Default)]
struct ViewCallStatesData {
    view_call_states: HashMap<CallStack, Mutable<RunState>>,
    call_states: ThreadCallStates,
}
