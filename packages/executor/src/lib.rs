use indoc::indoc;

pub mod library;
pub mod run;
pub mod syntax_tree;

pub const CODE: &str = indoc! {r#"
    def main():
        if True:
            pass

        if True:
            pass
        else:
            pass

        if function1(function2(function3())):
            function1()
            print("false1")
        else:
            print("true1")

        if True:
            print("true2")

        if False:
            print("false2")


        function1()
        function2()
        function2()
        function3()

    def function1():
        function2(function3())
        print("Hello, world!")

    def function2():
        function3()
        function4()

    def function3():
        function4()

    def function4():
        pass
"#};
