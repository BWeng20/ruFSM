pub struct Fsm {
    pub version : String,
    pub initial : Option<State>,
    pub datamodel : String
}

impl Fsm {
    pub fn new() -> Fsm {
        Fsm {
            version: "1.0".to_string(),
            initial: None,
            datamodel: "ecmascript".to_string()
        }
    }
}

pub struct State {
    pub id : String,
    pub initial : Option<Box<State>>,
    pub states : Vec<Box<State>>,
    pub on_entry : Option<Box<ExecutableContent>>,
    pub on_exit : Option<Box<ExecutableContent>>,
    pub transition: Option<Box<Transition>>,
    pub parallel: Vec<Box<State>>,
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
            parallel: vec![]
        }
    }
}


pub struct Transition {
}

impl Transition {
    pub fn new() -> Transition {
        Transition {

        }
    }
}

pub struct ExecutableContent {

}

