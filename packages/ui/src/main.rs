use std::{
    iter,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use futures::StreamExt;
use futures_signals::signal::{Mutable, SignalExt};
use gloo_console::log;
use gloo_net::websocket::futures::WebSocket;
use itertools::chain;
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::FnStatus,
    syntax_tree::{parse, Expression, Function, Statement},
    CODE,
};
use silkenweb::{
    clone,
    elements::{
        html::{a, button, div, i, li, span, ul, DivBuilder, LiBuilder},
        AriaElement, ElementEvents,
    },
    mount,
    node::element::{Element, ElementBuilder},
    prelude::{HtmlElement, ParentBuilder},
};

mod bs {
    silkenweb::css_classes!(visibility: pub, path: "bootstrap.min.css");
}

mod css {
    silkenweb::css_classes!(visibility: pub, path: "serpent-automation.scss");
}

mod icon {
    silkenweb::css_classes!(visibility: pub, path: "bootstrap-icons.css");
}

const BUTTON_STYLE: &str = bs::BTN_OUTLINE_SECONDARY;

fn dropdown<'a>(
    name: &'a str,
    fn_status: FnStatus,
    classes: impl IntoIterator<Item = &'a str>,
) -> Element {
    static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

    let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    let id = format!("dropdown-{id}");

    let mut main_button = button()
        .class([bs::BTN, BUTTON_STYLE, bs::DROPDOWN_TOGGLE])
        .id(&id)
        .attribute("data-bs-toggle", "dropdown")
        .r#type("button")
        .aria_expanded("false");

    main_button = match fn_status {
        FnStatus::NotRun => main_button,
        FnStatus::Running => main_button
            .child(
                span()
                    .class([bs::SPINNER_BORDER, bs::SPINNER_BORDER_SM, bs::TEXT_PRIMARY])
                    .aria_hidden("true"),
            )
            .text(" "),
        FnStatus::Ok => main_button
            .child(i().class([bs::TEXT_SUCCESS, icon::BI_CHECK_CIRCLE_FILL]))
            .text(" "),
        FnStatus::Error => main_button
            .child(i().class([bs::TEXT_DANGER, icon::BI_EXCLAMATION_TRIANGLE_FILL]))
            .text(" "),
    };

    button_group(classes)
        .child(main_button.text(name))
        .child(
            ul().class([bs::DROPDOWN_MENU])
                .aria_labelledby(id)
                .children([dropdown_item("Run"), dropdown_item("Pause")]),
        )
        .into()
}

fn button_group<'a>(classes: impl IntoIterator<Item = &'a str>) -> DivBuilder {
    div()
        .class(classes.into_iter().chain([bs::BTN_GROUP]))
        .role("group")
}

fn dropdown_item(name: &str) -> LiBuilder {
    li().child(a().class([bs::DROPDOWN_ITEM]).href("#").text(name))
}

fn row<'a>(classes: impl IntoIterator<Item = &'a str>) -> DivBuilder {
    div().class(classes.into_iter().chain([bs::D_FLEX, bs::FLEX_ROW]))
}

fn column<'a>(classes: impl IntoIterator<Item = &'a str>) -> DivBuilder {
    div().class(classes.into_iter().chain([bs::D_FLEX, bs::FLEX_COLUMN]))
}

fn end() -> Element {
    dropdown("end", FnStatus::Ok, [bs::SHADOW])
}

fn horizontal_line() -> Element {
    div().class([css::HORIZONTAL_LINE]).into()
}

fn arrow_right() -> Element {
    div()
        .class([css::ARROW_HEAD_RIGHT, bs::BG_SECONDARY])
        .into()
}

fn render_call(
    name: FunctionId,
    args: &[Expression<FunctionId>],
    library: &Rc<Library>,
) -> Vec<Element> {
    args.iter()
        .flat_map(|arg| render_expression(arg, library))
        .chain(iter::once(render_function(library.lookup(name), library)))
        .collect()
}

fn render_function(f: &Function<FunctionId>, library: &Rc<Library>) -> Element {
    let expanded = Mutable::new(false);
    let name = f.name();
    let main = row([bs::ALIGN_ITEMS_CENTER])
        .child(render_function_header(name, expanded.clone()))
        .child(horizontal_line())
        .child(arrow_right());

    let library = library.clone();
    let body = f.body().clone();

    column([bs::ALIGN_ITEMS_STRETCH])
        .child(main)
        .optional_child_signal(
            expanded
                .signal()
                .map(move |expanded| expanded.then(|| render_function_body(body.iter(), &library))),
        )
        .into()
}

fn render_function_body<'a>(
    body: impl Iterator<Item = &'a Statement<FunctionId>>,
    library: &Rc<Library>,
) -> DivBuilder {
    let border = [bs::BORDER, bs::BORDER_SECONDARY, bs::ROUNDED, bs::SHADOW];
    let box_model = [bs::MT_3, bs::ME_3, bs::P_3];

    row(chain!(
        [
            bs::ALIGN_SELF_START,
            bs::ALIGN_ITEMS_START,
            css::SPEECH_BUBBLE_BELOW,
        ],
        border,
        box_model
    ))
    .children(body.flat_map(|statement| match statement {
        Statement::Pass => Vec::new(),
        Statement::Expression(expr) => render_expression(expr, library),
    }))
    .child(end())
}

fn render_function_header(name: &str, expanded: Mutable<bool>) -> DivBuilder {
    button_group([bs::SHADOW])
        .aria_label(format!("Function {name}"))
        .child(dropdown(name, FnStatus::Ok, []))
        .child(
            button()
                .on_click({
                    clone!(expanded);
                    move |_, _| {
                        expanded.replace_with(|e| !*e);
                    }
                })
                .r#type("button")
                .class([bs::BTN, BUTTON_STYLE])
                .child(i().class_signal(expanded.signal().map(|expanded| {
                    [if expanded {
                        icon::BI_ZOOM_OUT
                    } else {
                        icon::BI_ZOOM_IN
                    }]
                }))),
        )
}

fn render_expression(expr: &Expression<FunctionId>, library: &Rc<Library>) -> Vec<Element> {
    match expr {
        Expression::Variable { .. } => Vec::new(),
        Expression::Call { name, args } => render_call(*name, args, library),
    }
}

fn main() {
    let module = parse(CODE).unwrap();
    let library = Rc::new(Library::link(module));
    let mut ws = WebSocket::open("ws://127.0.0.1:9090/").unwrap();
    let ws_handler = async move {
        while let Some(msg) = ws.next().await {
            log!(format!("Received: {:?}", msg))
        }

        log!("WebSocket Closed")
    };

    let app = row([
        css::FLOW_DIAGRAMS_CONTAINER,
        bs::M_3,
        bs::ALIGN_ITEMS_START,
        bs::OVERFLOW_AUTO,
    ])
    .children([render_function(library.main().unwrap(), &library), end()])
    .spawn_future(ws_handler);

    mount("app", app);
}
