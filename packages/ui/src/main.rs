use serpent_automation_ui::app;
use silkenweb::mount;

fn main() {
    mount("app", app());
}
