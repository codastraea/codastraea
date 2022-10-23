use indoc::indoc;

pub mod library;
pub mod run;
pub mod syntax_tree;

// TODO: Raw string
pub const CODE: &str = indoc! {"
    def main():
        if True:
            print(\"true0\")
        else:
            print(\"false0\")

        if False: print(\"false1\")
        else: print(\"true1\")

        if True:
            print(\"true2\")

        if False:
            print(\"false2\")


        function1()
        function2()
        function3()

    def function1():
        function2(function3())
        print(\"Hello, world!\")

    def function2():
        function3()
        function4()

    def function3():
        function4()

    def function4():
        pass
"};
