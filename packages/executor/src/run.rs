use std::{thread::sleep, time::Duration};

use crate::{
    library::{FunctionId, Library},
    syntax_tree::{Expression, Function, Statement},
};

type CallStack = Vec<FunctionId>;

#[derive(Default)]
pub struct RunTracer {
    running: CallStack,
    completed: Vec<CallStack>,
}

impl RunTracer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, id: FunctionId) {
        self.running.push(id);
        sleep(Duration::from_secs(1));
    }

    pub fn pop(&mut self) {
        self.completed.push(self.running.clone());
        self.running.pop();
    }
}
pub enum FnStatus {
    NotRun,
    Running,
    Ok,
    Error,
}

pub trait Run {
    fn run(&self, lib: &Library, tracer: &mut RunTracer);
}

pub fn run(lib: &Library, tracer: &mut RunTracer) {
    if let Some(main) = lib.main() {
        println!("Running main");
        main.run(lib, tracer);
        println!("Run finished");
    }
}

impl Run for Function<FunctionId> {
    fn run(&self, lib: &Library, tracer: &mut RunTracer) {
        println!("Running function '{}'", self.name());

        for stmt in self.body().iter() {
            stmt.run(lib, tracer)
        }
    }
}

impl Run for Statement<FunctionId> {
    fn run(&self, lib: &Library, tracer: &mut RunTracer) {
        match self {
            Statement::Pass => println!("pass"),
            Statement::Expression(expr) => expr.run(lib, tracer),
        }
    }
}

impl Run for Expression<FunctionId> {
    fn run(&self, lib: &Library, tracer: &mut RunTracer) {
        match self {
            Expression::Variable { name } => println!("Variable {name}"),
            Expression::Call { name, .. } => {
                tracer.push(*name);
                lib.lookup(*name).run(lib, tracer);
                tracer.pop();
            }
        }
    }
}
