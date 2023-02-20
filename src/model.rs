use std::collections::HashMap;
use std::fmt::{Debug};

pub type Id = String;

#[derive(Debug)]
pub struct Fsm {
    pub version : String,
    pub initial : Option<Id>,
    pub datamodel : String,

    /**
     * The only real storage to states, identified by the Id
     * If a state has no declared id, it needs a generated one.
     */
    pub states : HashMap<Id, State>

}

impl Fsm {
    pub fn new() -> Fsm {
        Fsm {
            version: "1.0".to_string(),
            initial: None,
            datamodel: "ecmascript".to_string(),
            states: Default::default()
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub id : String,
    pub initial : Option<Id>,
    pub states : Vec<Id>,
    pub on_entry : Option<ExecutableContent>,
    pub on_exit : Option<ExecutableContent>,
    pub transition: Option<Transition>,
    pub parallel: Vec<Id>,
    pub datamodel: Option<DataModel>,
}

#[derive(Debug)]
pub struct Data {
    // TODO ???
}

#[derive(Debug)]
pub struct DataModel {
    pub values : HashMap<String, Data>
}

impl State {
    pub fn new(id : &str) -> State {
        State {
            id:  id.to_string(),
            initial: None,
            states: vec![],
            on_entry: None,
            on_exit: None,
            transition: None,
            parallel: vec![],
            datamodel: None
        }
    }
}

#[derive(Debug)]
pub enum TransitionType {
    Internal,
    External
}

#[derive(Debug)]
pub struct Transition {
    // TODO: Possibly we need some type to express event ids
    pub events: Vec<String>,
    pub cond: Option<Box<dyn ConditionalExpression>>,
    pub target: Option<Id>,
    pub transition_type : Option<TransitionType>

}

impl Transition {
    pub fn new() -> Transition {
        Transition {
            events: vec![],
            cond: None,
            target: None,
            transition_type: None
        }
    }
}

/**
 * A boolean expression, implemented in the  used datamodel-language.
 */
trait ConditionalExpression: Debug {
    fn execute(self: &Self, data : DataModel) -> bool { false }
}

#[derive(Debug)]
pub struct ExecutableContent {

}

