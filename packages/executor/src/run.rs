use std::{collections::HashSet, sync::RwLock, thread::sleep, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    library::{FunctionId, Library},
    syntax_tree::{Expression, Function, Statement},
};

type CallStack = Vec<FunctionId>;

#[derive(Default, Serialize, Deserialize)]
pub struct RunTracer {
    running: CallStack,
    completed: HashSet<CallStack>,
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
        self.completed.insert(self.running.clone());
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
    fn run(&self, lib: &Library, tracer: &RwLock<RunTracer>);
}

pub fn run(lib: &Library, tracer: &RwLock<RunTracer>) {
    if let Some(main) = lib.main() {
        println!("Running main");
        main.run(lib, tracer);
        println!("Run finished");
    }
}

impl Run for Function<FunctionId> {
    fn run(&self, lib: &Library, tracer: &RwLock<RunTracer>) {
        println!("Running function '{}'", self.name());

        for stmt in self.body().iter() {
            stmt.run(lib, tracer)
        }
    }
}

impl Run for Statement<FunctionId> {
    fn run(&self, lib: &Library, tracer: &RwLock<RunTracer>) {
        match self {
            Statement::Pass => println!("pass"),
            Statement::Expression(expr) => expr.run(lib, tracer),
        }
    }
}

impl Run for Expression<FunctionId> {
    fn run(&self, lib: &Library, tracer: &RwLock<RunTracer>) {
        match self {
            Expression::Variable { name } => println!("Variable {name}"),
            Expression::Call { name, .. } => {
                tracer.write().unwrap().push(*name);
                lib.lookup(*name).run(lib, tracer);
                tracer.write().unwrap().pop();
            }
        }
    }
}
