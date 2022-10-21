use std::{cell::RefCell, collections::HashMap, rc::Rc};

use futures::StreamExt;
use futures_signals::signal::{Mutable, Signal};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use serpent_automation_executor::{
    library::FunctionId,
    run::{CallStack, FnStatus, ThreadState},
    syntax_tree::{Expression, Statement},
};

fn expression_is_expandable(expression: &Expression<FunctionId>) -> bool {
    match expression {
        Expression::Variable { .. } => false,
        Expression::Call { .. } => true,
    }
}

pub fn statement_is_expandable(stmt: &Statement<FunctionId>) -> bool {
    match stmt {
        Statement::Pass => false,
        Statement::Expression(e) => expression_is_expandable(e),
    }
}

pub fn is_expandable(stmts: &[Statement<FunctionId>]) -> bool {
    stmts.iter().any(statement_is_expandable)
}

pub async fn server_connection(stack_frame_states: StackFrameStates) {
    log!("Connecting to websocket");
    let mut server_ws = WebSocket::open("ws://127.0.0.1:9090/").unwrap();

    while let Some(msg) = server_ws.next().await {
        log!(format!("Received: {:?}", msg));

        match msg.unwrap() {
            Message::Text(text) => {
                let thread_state: ThreadState = serde_json_wasm::from_str(&text).unwrap();
                log!(format!("Deserialized `RunTracer` from `{text}`"));
                stack_frame_states.set_thread_state(thread_state);
            }
            Message::Bytes(_) => log!("Unknown binary message"),
        }
    }

    log!("WebSocket Closed")
}

#[derive(Clone, Default)]
pub struct StackFrameStates(Rc<RefCell<StackFrameStatesData>>);

impl StackFrameStates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self, call_stack: &CallStack) -> impl Signal<Item = FnStatus> {
        let mut data = self.0.borrow_mut();

        if let Some(existing) = data.stack_frame_states.get(call_stack) {
            existing
        } else {
            let new = Mutable::new(data.thread_state.status(call_stack));
            data.stack_frame_states
                .entry(call_stack.clone())
                .or_insert(new)
        }
        .signal()
    }

    fn set_thread_state(&self, thread_state: ThreadState) {
        let mut data = self.0.borrow_mut();

        for (call_stack, status) in &data.stack_frame_states {
            log!(format!("call stack {:?}", call_stack));
            status.set_neq(thread_state.status(call_stack));
        }

        data.thread_state = thread_state;
    }
}

#[derive(Default)]
struct StackFrameStatesData {
    stack_frame_states: HashMap<CallStack, Mutable<FnStatus>>,
    thread_state: ThreadState,
}
