use indoc::indoc;

pub mod library;
pub mod run;
pub mod syntax_tree;

pub const CODE: &str = indoc! {"
    def main():
        function1()
        function2()
        function3()

    def function1():
        function2(function3())

    def function2():
        function3()
        function4()

    def function3():
        function4()

    def function4():
        pass
"};
