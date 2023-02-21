use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

use crate::model::TransitionType::{External, Internal};

pub type Id = String;
pub type StateRef = Rc<RefCell<State>>;
pub type StateMap = HashMap<Id, StateRef>;

#[derive(Debug)]
pub struct Fsm {
    pub version: String,
    pub initial: Option<Id>,
    pub datamodel: String,

    /**
     * The only real storage to states, identified by the Id
     * If a state has no declared id, it needs a generated one.
     */
    pub states: StateMap,
}

fn display_state_map(sm: &StateMap, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{{")?;

    let mut first = true;
    for e in sm {
        if first {
            first = false;
        } else {
            write!(f, ",")?;
        }
        write!(f, "{}", (**e.1).borrow())?;
    }

    write!(f, "}}")
}

fn display_ids(sm: &Vec<Id>, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "[")?;

    let mut first = true;
    for e in sm {
        if first {
            first = false;
        } else {
            write!(f, ",")?;
        }
        write!(f, "{}", e)?;
    }

    write!(f, "]")
}

impl Fsm {
    pub fn new() -> Fsm {
        Fsm {
            version: "1.0".to_string(),
            initial: None,
            datamodel: "ecmascript".to_string(),
            states: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub id: String,
    pub initial: Option<Id>,
    pub initial_transition: Option<Transition>,
    pub states: Vec<Id>,
    pub on_entry: Option<ExecutableContent>,
    pub on_exit: Option<ExecutableContent>,
    pub transitions: Vec<Transition>,
    pub parallel: Vec<Id>,
    pub datamodel: Option<DataModel>,
}

#[derive(Debug)]
pub struct Data {
    // TODO ???
}

#[derive(Debug)]
pub struct DataModel {
    pub values: HashMap<String, Data>,
}

impl State {
    pub fn new(id: &str) -> State {
        State {
            id: id.to_string(),
            initial: None,
            initial_transition: None,
            states: vec![],
            on_entry: None,
            on_exit: None,
            transitions: vec![],
            parallel: vec![],
            datamodel: None,
        }
    }
}

#[derive(Debug)]
pub enum TransitionType {
    Internal,
    External,
}

pub fn map_transition_type(ts: &String) -> Option<TransitionType> {
    let mut t: Option<TransitionType> = None;
    match ts.to_lowercase().as_str() {
        "internal" => t = Some(Internal),
        "external" => t = Some(External),
        "" => {}
        _ => panic!("Unknown transition type '{}'", ts)
    }
    t
}


#[derive(Debug)]
pub struct Transition {
    // TODO: Possibly we need some type to express event ids
    pub events: Vec<String>,
    pub cond: Option<Box<dyn ConditionalExpression>>,
    pub target: Option<Id>,
    pub transition_type: Option<TransitionType>,
}

impl Transition {
    pub fn new() -> Transition {
        Transition {
            events: vec![],
            cond: None,
            target: None,
            transition_type: None,
        }
    }
}

/**
 * A boolean expression, implemented in the  used datamodel-language.
 */
pub trait ConditionalExpression: Debug {
    fn execute(self: &Self, data: DataModel) -> bool { false }
}

#[derive(Debug)]
pub struct ScriptConditionalExpression {
    pub script: String,
}

impl ScriptConditionalExpression {
    pub fn new(s: &String) -> ScriptConditionalExpression {
        ScriptConditionalExpression {
            script: s.clone()
        }
    }
}

impl ConditionalExpression for ScriptConditionalExpression {
    fn execute(self: &Self, data: DataModel) -> bool {
        return true;
    }
}


#[derive(Debug)]
pub struct ExecutableContent {}

////////////////////////////////////////
//// Display support

// Returns the id or "none"
fn optional_to_string<T: Display>(op: &Option<T>) -> String {
    if op.is_some() {
        format!("{}", op.as_ref().unwrap())
    } else {
        "none".to_string()
    }
}

impl Display for Fsm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fsm{{v:{} initial:{} states:", self.version, optional_to_string(&self.initial))?;
        display_state_map(&self.states, f)?;
        write!(f, "}}")
    }
}


impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{<{}> initial:", self.id)?;
        if self.initial_transition.is_some() {
            write!(f, "{}", optional_to_string(&self.initial_transition))?;
        } else if self.initial.is_some() {
            write!(f, "{}", optional_to_string(&self.initial))?;
        } else {
            write!(f, "none")?;
        }
        write!(f, " states:")?;
        display_ids(&self.states, f)?;
        write!(f, "}}")
    }
}

impl Display for Transition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{type:{} events:{:?} cond:{:?} target:{} }}",
               optional_to_string(&self.transition_type), &self.events, self.cond,
               optional_to_string(&self.target))
    }
}

impl Display for TransitionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Internal => f.write_str("internal"),
            External => f.write_str("external")
        }
    }
}


