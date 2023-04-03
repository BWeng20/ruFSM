use std::fmt::Debug;

use lazy_static::lazy_static;
use log::warn;
use regex::Regex;

use crate::{Event, EventType};
use crate::fsm::{Datamodel, ExecutableContentId, Fsm};

pub const TARGET_INTERNAL: &str = "_internal";

/// a duration.
/// RegExp: "\\d*(\\.\\d+)?(ms|s|m|h|d))").
pub fn parse_duration_to_millies(d: &String) -> i64 {
    lazy_static! {
        static ref DURATION_RE: Regex = Regex::new(r"^(\d*(\.\d+)?)(MS|S|M|H|D|ms|s|m|h|d)$").unwrap();
    }

    let caps = DURATION_RE.captures(d);
    if caps.is_none() {
        -1
    } else {
        let cap = caps.unwrap();
        let value = cap.get(1).map_or("", |m| m.as_str());
        let unit = cap.get(3).map_or("", |m| m.as_str());

        if value.is_empty() {
            -1
        } else {
            let mut v: f64 = value.parse::<f64>().unwrap();
            match unit {
                "D" | "d" => {
                    v = v * 24.0 * 60.0 * 60.0 * 1000.0;
                }
                "H" | "h" => {
                    v = v * 60.0 * 60.0 * 1000.0;
                }
                "M" | "m" => {
                    v = v * 60000.0;
                }
                "S" | "s" => {
                    v = v * 1000.0;
                }
                "MS" | "ms" => {}
                _ => {
                    return -1;
                }
            }
            v.round() as i64
        }
    }
}

pub trait ExecutableContent: Debug + Send {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm);
}

#[derive(Debug)]
pub struct Script {
    pub content: Vec<ExecutableContentId>,
}

#[derive(Debug)]
pub struct Expression {
    pub content: String,
}

#[derive(Debug)]
pub struct Log {
    pub expression: String,
}

#[derive(Debug)]
pub struct If {
    pub expression: String,
    pub content: ExecutableContentId,
    pub else_content: ExecutableContentId,
}

#[derive(Debug)]
pub struct ForEach {
    pub array: String,
    pub item: String,
    pub index: Option<String>,
    pub content: ExecutableContentId,
}

#[derive(Debug)]
pub struct SendParameters {
    pub namelocation: String,
    /// The SCXML id.
    pub name: String,

    pub event: String,
    pub eventexpr: String,
    pub target: String,
    pub targetexpr: String,
    pub typeS: String,
    pub typeexpr: String,

    pub delay: String,
    pub delayexrp: String,

    pub nameList: String,

    pub content: ExecutableContentId,
}


impl Script {
    pub fn new() -> Script {
        Script {
            content: Vec::new(),
        }
    }
}

impl ExecutableContent for Script {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) {
        for s in &self.content {
            let _l = datamodel.executeContent(fsm, *s);
        }
    }
}

impl Expression {
    pub fn new(expression: String) -> Expression {
        Expression {
            content: expression,
        }
    }
}

impl ExecutableContent for Expression {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) {
        let _l = datamodel.execute(fsm, &self.content);
    }
}

impl Log {
    pub fn new(expression: &str) -> Log {
        Log {
            expression: expression.to_string(),
        }
    }
}

impl ExecutableContent for Log {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) {
        let l = datamodel.execute(fsm, &self.expression);
        datamodel.log(&l);
    }
}

impl If {
    pub fn new(expression: &str) -> If {
        If {
            expression: expression.to_string(),
            content: 0,
            else_content: 0,
        }
    }
}

impl ExecutableContent for If {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) {
        match datamodel.executeCondition(fsm, &self.expression) {
            Ok(r) => {
                if r {
                    if self.content != 0 {
                        for e in fsm.executableContent.get(&self.content).unwrap() {
                            e.execute(datamodel, fsm);
                        }
                    }
                } else {
                    if self.else_content != 0 {
                        for e in fsm.executableContent.get(&self.else_content).unwrap() {
                            e.execute(datamodel, fsm);
                        }
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
        ForEach {
            array: array.to_string(),
            item: "".to_string(),
            index: None,
            content: 0,
        }
    }
}

impl ExecutableContent for ForEach {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) {
        let tmp = INDEX_TEMP.to_string();
        let idx = self.index.as_ref().unwrap_or_else(|| { &tmp });
        datamodel.executeForEach(fsm, &self.array, &self.item, &idx, &|fsmI: &mut Fsm, dm: &mut dyn Datamodel| {
            if self.content != 0 {
                for e in fsmI.executableContent.get(&self.content).unwrap() {
                    e.execute(dm, fsmI);
                }
            }
        });
    }
}

impl SendParameters {
    pub fn new() -> SendParameters {
        SendParameters {
            namelocation: "".to_string(),
            name: "".to_string(),
            event: "".to_string(),
            eventexpr: "".to_string(),
            target: "".to_string(),
            targetexpr: "".to_string(),
            typeS: "".to_string(),
            typeexpr: "".to_string(),
            delay: "".to_string(),
            delayexrp: "".to_string(),
            nameList: "".to_string(),
            content: 0,
        }
    }
}

impl ExecutableContent for SendParameters {
    /// If unable to dispatch, place "error.communication" in internal queue
    /// If target is not supported, place "error.execution" in internal queue
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) {
        let target;
        if self.target.is_empty() {
            target = datamodel.execute(fsm, &self.targetexpr);
        } else {
            target = self.target.clone();
        }
        let sender = datamodel.global().externalQueue.sender.clone();

        if target.is_empty()
        {
            datamodel.global().internalQueue.enqueue(Event::error("execution"));
        } else {
            let delay;
            if !self.delayexrp.is_empty() {
                delay = datamodel.execute(fsm, &self.delayexrp);
            } else {
                delay = self.delay.clone();
            }
            if delay.is_empty() {
                todo!()
            } else {
                if target.eq(TARGET_INTERNAL) {
                    // Can't send timers via internal queue
                    datamodel.global().internalQueue.enqueue(Event::error("execution"));
                } else {
                    let timer = timer::Timer::new();

                    let _guard = timer.schedule_with_delay(chrono::Duration::seconds(3), move || {
                        let event = Box::new(Event {
                            name: "TODO".to_string(),
                            etype: EventType::external,
                            sendid: 0,
                            origin: "".to_string(),
                            origintype: "".to_string(),
                            invokeid: 0,
                            data: None,
                        });
                        sender.send(event);
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::executable_content::parse_duration_to_millies;

    #[test]
    fn delay_parse() {
        assert_eq!(parse_duration_to_millies(&"6.7s".to_string()), 6700);
        assert_eq!(parse_duration_to_millies(&"0.5d".to_string()), 12 * 60 * 60 * 1000);
        assert_eq!(parse_duration_to_millies(&"1m".to_string()), 60 * 1000);
        assert_eq!(parse_duration_to_millies(&"0.001s".to_string()), 1);
        assert_eq!(parse_duration_to_millies(&"6.7S".to_string()), 6700);
        assert_eq!(parse_duration_to_millies(&"0.5D".to_string()), 12 * 60 * 60 * 1000);
        assert_eq!(parse_duration_to_millies(&"1M".to_string()), 60 * 1000);
        assert_eq!(parse_duration_to_millies(&"0.001S".to_string()), 1);

        assert_eq!(parse_duration_to_millies(&"x1S".to_string()), -1);
        assert_eq!(parse_duration_to_millies(&"1Sx".to_string()), -1);
    }
}