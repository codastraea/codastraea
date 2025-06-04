use codastraea_ui::app;
use silkenweb::mount;

fn main() {
    mount("app", app());
}
