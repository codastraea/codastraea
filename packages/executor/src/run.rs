use std::{collections::HashSet, sync::RwLock, thread::sleep, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    library::{FunctionId, Library},
    syntax_tree::{Expression, Function, Statement},
};

pub type CallStack = Vec<FunctionId>;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct RunTracer {
    running: CallStack,
    completed: HashSet<CallStack>,
}

impl RunTracer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self, call_stack: &CallStack) -> FnStatus {
        if self.running.starts_with(call_stack) {
            FnStatus::Running
        } else if self.completed.contains(call_stack) {
            FnStatus::Ok
        } else {
            FnStatus::NotRun
        }
    }

    pub fn push(&mut self, id: FunctionId) {
        self.running.push(id);
    }

    pub fn pop(&mut self) {
        self.completed.insert(self.running.clone());
        self.running.pop();
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
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
    if let Some(main_id) = lib.main_id() {
        Expression::Call {
            name: main_id,
            args: Vec::new(),
        }
        .run(lib, tracer);
    }
}

impl Run for Function<FunctionId> {
    fn run(&self, lib: &Library, tracer: &RwLock<RunTracer>) {
        println!("Running function '{}'", self.name());
        sleep(Duration::from_secs(2));

        for stmt in self.body().iter() {
            stmt.run(lib, tracer)
        }
    }
}

impl Run for Statement<FunctionId> {
    fn run(&self, lib: &Library, tracer: &RwLock<RunTracer>) {
        match self {
            Statement::Pass => (),
            Statement::Expression(expr) => expr.run(lib, tracer),
        }
    }
}

impl Run for Expression<FunctionId> {
    fn run(&self, lib: &Library, tracer: &RwLock<RunTracer>) {
        match self {
            Expression::Variable { name } => println!("Variable {name}"),
            Expression::Call { name, args } => {
                for arg in args {
                    arg.run(lib, tracer);
                }

                tracer.write().unwrap().push(*name);
                lib.lookup(*name).run(lib, tracer);
                tracer.write().unwrap().pop();
            }
        }
    }
}
