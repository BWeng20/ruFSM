use std::borrow::BorrowMut;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering;

use log::{info, warn};

use crate::fsm::{Datamodel, ExecutableContentId, Fsm, ID_COUNTER};
use crate::reader::TAG_LOG;

pub trait ExecutableContent: Debug + Send {
    fn Id(&self) -> ExecutableContentId;
    fn execute(&self, datamodel: &mut dyn Datamodel);
}

#[derive(Debug)]
pub struct Script {
    pub id: ExecutableContentId,
    pub content: Vec<ExecutableContentId>,
}

#[derive(Debug)]
pub struct Expression {
    pub id: ExecutableContentId,
    pub content: String,
}

#[derive(Debug)]
pub struct Log {
    pub id: ExecutableContentId,
    pub expression: String,
}

#[derive(Debug)]
pub struct If {
    pub id: ExecutableContentId,
    pub expression: String,
    pub content: ExecutableContentId,
    pub else_content: ExecutableContentId,
}

#[derive(Debug)]
pub struct ForEach {
    pub id: ExecutableContentId,
    pub array: String,
    pub item: String,
    pub index: Option<String>,
    pub content: ExecutableContentId,
}

impl Script {
    pub fn new() -> Script {
        let idc = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Script {
            id: idc,
            content: Vec::new(),
        }
    }
}

impl ExecutableContent for Script {
    fn Id(&self) -> ExecutableContentId {
        self.id
    }

    fn execute(&self, datamodel: &mut dyn Datamodel) {
        for s in &self.content {
            let l = datamodel.executeContent(*s);
        }
    }
}

impl Expression {
    pub fn new(expression: String) -> Expression {
        let idc = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Expression {
            id: idc,
            content: expression,
        }
    }
}

impl ExecutableContent for Expression {
    fn Id(&self) -> ExecutableContentId {
        self.id
    }

    fn execute(&self, datamodel: &mut dyn Datamodel) {
        let l = datamodel.execute(&self.content);
    }
}

impl Log {
    pub fn new(expression: &str) -> Log {
        let idc = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Log {
            id: idc,
            expression: expression.to_string(),
        }
    }
}

impl ExecutableContent for Log {
    fn Id(&self) -> ExecutableContentId {
        self.id
    }

    fn execute(&self, datamodel: &mut dyn Datamodel) {
        let l = datamodel.execute(&self.expression);
        datamodel.log(&l);
    }
}

impl If {
    pub fn new(expression: &str) -> If {
        let idc = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        If {
            id: idc,
            expression: expression.to_string(),
            content: 0,
            else_content: 0,
        }
    }
}

impl ExecutableContent for If {
    fn Id(&self) -> ExecutableContentId {
        self.id
    }

    fn execute(&self, datamodel: &mut dyn Datamodel) {
        let mut global = datamodel.global();
        let mut ex = global.deref().borrow_mut();
        match datamodel.executeCondition(&self.expression) {
            Ok(r) => {
                if r {
                    if self.content != 0 {
                        ex.executableContent.get(&self.content).unwrap().execute(datamodel);
                    }
                } else {
                    if self.else_content != 0 {
                        ex.executableContent.get(&self.else_content).unwrap().execute(datamodel);
                    }
                }
            }
            Err(e) => {
                warn!("Condition {} can't be evaluated. {}", self.expression, e);
            }
        }
    }
}

pub const INDEX_TEMP: &str = "__$index";

impl ForEach {
    pub fn new(array: &str) -> ForEach {
        let idc = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        ForEach {
            id: idc,
            array: array.to_string(),
            item: "".to_string(),
            index: None,
            content: 0,
        }
    }
}

impl ExecutableContent for ForEach {
    fn Id(&self) -> ExecutableContentId {
        self.id
    }

    fn execute(&self, datamodel: &mut dyn Datamodel) {
        let tmp = INDEX_TEMP.to_string();
        let idx = self.index.as_ref().unwrap_or_else(|| { &tmp });
        let mut global = datamodel.global();
        let mut ex = global.deref().borrow_mut();
        datamodel.executeForEach(&self.array, &self.item, &idx, &|md: &mut dyn Datamodel| {
            if self.content != 0 {
                ex.executableContent.get(&self.content).unwrap().execute(md);
            }
        });
    }
}
