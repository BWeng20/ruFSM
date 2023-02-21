use std::cell::RefCell;
use std::collections::{HashMap, LinkedList};
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

pub const ECMA_SCRIPT: &str = "ECMAScript";
pub const ECMA_SCRIPT_LC: &str = "ecmascript";


////////////////////////////////////////////////////////////////////////////////
// Implementation of the data-structures and algorithms described in the W3C
// scxml proposal.
// As reference each type and method have the w3c description as documentation.
// See https://www.w3.org/TR/scxml/#AlgorithmforSCXMLInterpretation

////////////////////////////////////////////////////////////////////////////////
// General Purpose Data types
// Structs and methods are designed to match the signatures in the W3c-Pseudo-code.

pub struct List<T> {
    data: LinkedList<Rc<RefCell<T>>>,
}

impl<T> List<T> {
    pub fn new() -> List<T> {
        List { data: Default::default() }
    }


    // Returns the head of the list
    pub fn head(&self) -> &Rc<RefCell<T>> {
        self.data.front().unwrap()
    }

    // Returns the tail of the list (i.e., the rest of the list once the head is removed)
    pub fn tail(&self) -> List<T> {
        let mut t = List {
            data: self.data.clone()
        };
        t.data.pop_front();
        t
    }

    // Returns the list appended with l
    pub fn append(&self, l: &List<T>) -> List<T> {
        let mut t = List {
            data: self.data.clone()
        };
        for i in l.data.iter()
        {
            t.data.push_back((*i).clone());
        }
        t
    }

    // Returns the list of elements that satisfy the predicate f
    pub fn filter(&self, f: &dyn Fn(&T) -> bool) -> List<T> {
        let mut t = List::new();

        for i in self.data.iter() {
            if f(&(**i).borrow()) {
                t.data.push_back((*i).clone());
            }
        }
        t
    }

    // Returns true if some element in the list satisfies the predicate f.  Returns false for an empty list.
    pub fn some(f: &dyn Fn(&T) -> bool) -> bool {
        false
    }
    // Returns true if every element in the list satisfies the predicate f.  Returns true for an empty list.
    pub fn every(f: &dyn Fn(&T) -> bool) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct OrderedSet<T> {
    data: Vec<Rc<RefCell<T>>>,
}

impl<T> OrderedSet<T> {
    pub fn new() -> OrderedSet<T> {
        OrderedSet { data: Default::default() }
    }

    // Adds e to the set if it is not already a member
    pub fn add(&mut self, e: &Rc<RefCell<T>>) {
        self.data.push(e.clone());
    }

    // Deletes e from the set
    pub fn delete(&mut self, e: &Rc<RefCell<T>>) { todo!() }

    // Adds all members of s that are not already members of the set (s must also be an OrderedSet)
    pub fn union(s: &OrderedSet<T>) { todo!() }

    // Is e a member of set?
    pub fn isMember(e: &Rc<RefCell<T>>) {
        todo!()
    }

    // Returns true if some element in the set satisfies the predicate f.  Returns false for an empty set.
    pub fn some(f: &dyn Fn(&T) -> bool) -> bool {
        todo!()
    }
    // Returns true if every element in the set satisfies the predicate f. Returns true for an empty set.
    pub fn every(f: &dyn Fn(&T) -> bool) -> bool {
        todo!()
    }
    // Returns true if this set and set s have at least one member in common
    pub fn hasIntersection(s: &OrderedSet<T>) {
        todo!()
    }

    // Is the set empty?
    pub fn isEmpty(&self) -> bool {
        todo!()
    }
    // Remove all elements from the set (make it empty)
    pub fn clear(&mut self) {
        todo!()
    }

    /**
     * Converts the set to a list that reflects the order in which elements were originally added
     * In the case of sets created by intersection, the order of the first set (the one on which the method was called) is used
     * In the case of sets created by union, the members of the first set (the one on which union was called) retain their original ordering
     * while any members belonging to the second set only are placed after, retaining their ordering in their original set.
     */
    pub fn toList(&self) -> List<T> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Queue<T> {
    data: Vec<Rc<RefCell<T>>>,
}

impl<T> Queue<T> {
    fn new() -> Queue<T> {
        Queue {
            data: vec![]
        }
    }
}

#[derive(Debug)]
pub struct BlockingQueue<T> {
    data: Vec<Rc<RefCell<T>>>,
}

impl<T> BlockingQueue<T> {
    fn new() -> BlockingQueue<T> {
        BlockingQueue {
            data: vec![]
        }
    }
}


#[derive(Debug)]
pub struct HashTable<K, T> {
    data: HashMap<K, Rc<RefCell<T>>>,
}

impl<K, T> HashTable<K, T> {
    fn new() -> HashTable<K, T> {
        HashTable { data: HashMap::new() }
    }
}

/////////////////////////////////////////////////////////////
// FSM model (State etc, representing the XML-data-model)

pub type Id = String;
pub type StateRef = Rc<RefCell<State>>;
pub type StateMap = HashMap<Id, StateRef>;

#[derive(Debug)]
pub enum BindingType {
    Early,
    Late,
}

#[derive(Debug)]
pub struct Event {
    pub name: String,
}

#[derive(Debug)]
pub struct Fsm {
    pub configuration: OrderedSet<State>,
    pub statesToInvoke: OrderedSet<State>,
    pub datamodel: Box<dyn Datamodel>,
    pub internalQueue: Queue<Event>,
    pub externalQueue: BlockingQueue<Event>,
    pub historyValue: HashTable<Id, State>,
    pub running: bool,
    pub binding: BindingType,

    pub version: String,
    pub initial: Option<Id>,

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
            configuration: OrderedSet::new(),
            version: "1.0".to_string(),
            initial: None,
            datamodel: createDatamodel(ECMA_SCRIPT),
            internalQueue: Queue::new(),
            externalQueue: BlockingQueue::new(),
            historyValue: HashTable::new(),
            running: false,
            statesToInvoke: OrderedSet::new(),
            binding: BindingType::Early,
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
        "internal" => t = Some(TransitionType::Internal),
        "external" => t = Some(TransitionType::External),
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

pub trait Datamodel: Debug {
    fn get_name(self: &Self) -> &str;
}

pub fn createDatamodel(name: &str) -> Box<dyn Datamodel> {
    match name.to_lowercase().as_str() {
        ECMA_SCRIPT_LC => Box::new(ECMAScript::new()),
        _ => panic!("Unsupported Datamodel '{}'", name)
    }
}

#[derive(Debug)]
pub struct ECMAScript {}

impl ECMAScript {
    pub fn new() -> ECMAScript {
        ECMAScript {}
    }
}

impl Datamodel for ECMAScript {
    fn get_name(self: &Self) -> &str {
        return ECMA_SCRIPT;
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
            TransitionType::Internal => f.write_str("internal"),
            TransitionType::External => f.write_str("external")
        }
    }
}


