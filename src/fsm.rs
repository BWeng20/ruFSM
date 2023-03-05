#![allow(non_snake_case)]

use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::slice::Iter;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

pub const ECMA_SCRIPT: &str = "ECMAScript";
pub const ECMA_SCRIPT_LC: &str = "ecmascript";

pub const NULL_DATAMODEL: &str = "NULL";
pub const NULL_DATAMODEL_LC: &str = "null";

fn entryOrder(s1: &StateId, s2: &StateId) -> ::std::cmp::Ordering {
    // Same as Document order
    if s1 > s2 {
        std::cmp::Ordering::Greater
    } else if s1 == s2 {
        std::cmp::Ordering::Equal
    } else { std::cmp::Ordering::Less }
}

fn exitOrder(s1: &StateId, s2: &StateId) -> ::std::cmp::Ordering {
    // Same as Document order
    if s1 < s2 {
        std::cmp::Ordering::Greater
    } else if s1 == s2 {
        std::cmp::Ordering::Equal
    } else { std::cmp::Ordering::Less }
}

fn documentOrder(s1: &StateId, s2: &StateId) -> ::std::cmp::Ordering {
    // TODO: Ids are generated also the first time a state is references, NOT only defined.
    // For document order we need a separate field
    if s1 > s2 {
        std::cmp::Ordering::Greater
    } else if s1 == s2 {
        std::cmp::Ordering::Equal
    } else { std::cmp::Ordering::Less }
}

fn transitionDocumentOrder(t1: &&Transition, t2: &&Transition) -> ::std::cmp::Ordering {
    // Ids are generated in reader-order
    if t1.id > t2.id {
        std::cmp::Ordering::Greater
    } else if t1.id == t2.id {
        std::cmp::Ordering::Equal
    } else { std::cmp::Ordering::Less }
}

fn invokeDocumentOrder(s1: &Invoke, s2: &Invoke) -> ::std::cmp::Ordering {
    // TODO: Ids are generated also the first time a state is references, NOT only defined.
    // For document order we need a separate field
    if s1.id > s2.id {
        std::cmp::Ordering::Greater
    } else if s1.id == s2.id {
        std::cmp::Ordering::Equal
    } else { std::cmp::Ordering::Less }
}

/// Starts the FSM inside a worker thread.
///
pub fn start_fsm(mut sm: Box<Fsm>) -> (JoinHandle<()>, Sender<Box<Event>>) {
    let externalQueue: BlockingQueue<Box<Event>> = BlockingQueue::new();
    let sender = externalQueue.sender.clone();
    let thread = thread::spawn(
        move || {
            println!("SM starting...");
            sm.externalQueue = externalQueue;
            sm.interpret();
            println!("SM finished");
        });
    (thread, sender)
}


////////////////////////////////////////////////////////////////////////////////
// ## Implementation of the data-structures and algorithms described in the W3C scxml proposal.
// As reference each type and method has the w3c description as documentation.
// See https://www.w3.org/TR/scxml/#AlgorithmforSCXMLInterpretation

////////////////////////////////////////////////////////////////////////////////
// ## General Purpose Data types
// Structs and methods are designed to match the signatures in the W3c-Pseudo-code.


/// ## General Purpose List type
#[derive(Clone)]
pub struct List<T: Clone> {
    data: Vec<T>,
}

impl<T: Clone + PartialEq> List<T> {
    pub fn new() -> List<T> {
        List { data: Default::default() }
    }

    pub fn from_array(l: &[T]) -> List<T> {
        List { data: l.to_vec() }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn push(&mut self, t: T) {
        self.data.push(t);
    }

    /// #W3C says:
    /// Returns the head of the list
    pub fn head(&self) -> &T {
        self.data.first().unwrap()
    }

    /// #W3C says:
    /// Returns the tail of the list (i.e., the rest of the list once the head is removed)
    pub fn tail(&self) -> List<T> {
        let mut t = List {
            data: self.data.clone()
        };
        t.data.remove(0);
        t
    }

    /// #W3C says:
    /// Returns the list appended with l
    pub fn append(&self, l: &List<T>) -> List<T> {
        let mut t = List {
            data: self.data.clone()
        };
        for i in l.data.iter()
        {
            t.data.push((*i).clone());
        }
        t
    }

    /// #W3C says:
    /// Returns the list appended with l
    pub fn appendSet(&self, l: &OrderedSet<T>) -> List<T> {
        let mut t = List {
            data: self.data.clone()
        };
        for i in l.data.iter()
        {
            t.data.push((*i).clone());
        }
        t
    }

    /// #W3C says:
    /// Returns the list of elements that satisfy the predicate f
    /// # Actual Implementation:
    /// Can't name the function "filter" because this get in conflict with pre-defined "filter"
    /// that is introduced by the Iterator-implementation.
    pub fn filterBy(&self, f: &dyn Fn(&T) -> bool) -> List<T> {
        let mut t = List::new();

        for i in self.data.iter() {
            if f(&(*i)) {
                t.data.push((*i).clone());
            }
        }
        t
    }

    /// #W3C says:
    /// Returns true if some element in the list satisfies the predicate f.  Returns false for an empty list.
    pub fn some(&self, f: &dyn Fn(&T) -> bool) -> bool {
        for si in &self.data {
            if f(si) {
                return true;
            }
        }
        false
    }

    /// #W3C says:
    /// Returns true if every element in the list satisfies the predicate f.  Returns true for an empty list.
    pub fn every(&self, f: &dyn Fn(&T) -> bool) -> bool {
        for si in &self.data {
            if !f(si) {
                return false;
            }
        }
        true
    }

    pub fn sort<F>(&self, compare: &F) -> List<T>
        where
            F: Fn(&T, &T) -> std::cmp::Ordering + ?Sized {
        let mut t = List {
            data: self.data.clone()
        };
        t.data.sort_by(compare);
        t
    }

    pub fn iterator(&self) -> Iter<'_, T> {
        self.data.iter()
    }

    pub fn toSet(&self) -> OrderedSet<T> {
        let mut s = OrderedSet::new();
        for e in self.data.iter()
        {
            s.add(e.clone());
        }
        s
    }
}


/// Set datatype used by the algorithm,
/// #W3C says:
/// Note that the algorithm assumes a Lisp-like semantics in which the empty Set null is equivalent
/// to boolean 'false' and all other entities are equivalent to 'true'.
///
/// The notation [...] is used as a list constructor, so that '[t]' denotes a list whose only member
/// is the object t.
#[derive(Debug)]
#[derive(Clone)]
pub struct OrderedSet<T> {
    data: Vec<T>,
}

impl<T: Clone + PartialEq> OrderedSet<T> {
    pub fn new() -> OrderedSet<T> {
        OrderedSet { data: Default::default() }
    }

    pub fn from_array(l: &[T]) -> OrderedSet<T> {
        OrderedSet { data: l.to_vec() }
    }

    /// Extension: The size (only informational)
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// #W3C says:
    /// Adds e to the set if it is not already a member
    pub fn add(&mut self, e: T) {
        self.data.push(e.clone());
    }

    /// #W3C says:
    /// Deletes e from the set
    pub fn delete(&mut self, e: &T) {
        self.data.retain(|x| *x != *e);
    }

    /// #W3C says:
    /// Adds all members of s that are not already members of the set
    /// (s must also be an OrderedSet)
    pub fn union(&mut self, s: &OrderedSet<T>) {
        for si in &s.data {
            if !self.isMember(&si) {
                self.add(si.clone());
            }
        }
    }

    /// #W3C says:
    /// Is e a member of set?
    pub fn isMember(&self, e: &T) -> bool {
        self.data.contains(e)
    }

    /// #W3C says:
    /// Returns true if some element in the set satisfies the predicate f.
    ///
    /// Returns false for an empty set.
    pub fn some(&self, f: &dyn Fn(&T) -> bool) -> bool {
        for si in &self.data {
            if f(si) {
                return true;
            }
        }
        false
    }

    /// #W3C says:
    /// Returns true if every element in the set satisfies the predicate f.
    ///
    /// Returns true for an empty set.
    pub fn every(&self, f: &dyn Fn(&T) -> bool) -> bool {
        for si in &self.data {
            if !f(si) {
                return false;
            }
        }
        true
    }

    /// #W3C says:
    /// Returns true if this set and set s have at least one member in common
    pub fn hasIntersection(&self, s: &OrderedSet<T>) -> bool {
        for si in &self.data {
            if s.isMember(si) {
                return true;
            }
        }
        false
    }

    /// #W3C says:
    /// Is the set empty?
    pub fn isEmpty(&self) -> bool {
        self.size() == 0
    }

    /// #W3C says:
    /// Remove all elements from the set (make it empty)
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// #W3C says:
    /// Converts the set to a list that reflects the order in which elements were originally added.
    ///
    /// In the case of sets created by intersection, the order of the first set (the one on which
    /// the method was called) is used
    ///
    /// In the case of sets created by union, the members of the first set (the one on which union
    /// was called) retain their original ordering while any members belonging to the second set only
    /// are placed after, retaining their ordering in their original set.
    pub fn toList(&self) -> List<T> {
        let mut l = List::new();
        for e in self.data.iter()
        {
            l.push(e.clone());
        }
        l
    }

    pub fn sort<F>(&self, compare: &F) -> List<T>
        where
            F: Fn(&T, &T) -> std::cmp::Ordering + ?Sized {
        let mut t = List {
            data: self.data.clone()
        };
        t.data.sort_by(compare);
        t
    }


    pub fn iterator(&self) -> Iter<'_, T> {
        self.data.iter()
    }
}

#[derive(Debug)]
pub struct Queue<T> {
    data: VecDeque<T>,
}

impl<T> Queue<T> {
    fn new() -> Queue<T> {
        Queue {
            data: VecDeque::new()
        }
    }

    /// Extension to re-use exiting instances.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// #W3C says:
    /// Puts e last in the queue
    pub fn enqueue(&mut self, e: T) {
        self.data.push_back(e);
    }

    /// #W3C says:
    /// Removes and returns first element in queue
    pub fn dequeue(&mut self) -> T {
        self.data.pop_front().unwrap()
    }

    /// #W3C says:
    /// Is the queue empty?
    pub fn isEmpty(&self) -> bool {
        self.data.is_empty()
    }
}

#[derive(Debug)]
pub struct BlockingQueue<T> {
    sender: Sender<T>,
    receiver: Arc<Mutex<Receiver<T>>>,
}

impl<T> BlockingQueue<T> {
    fn new() -> BlockingQueue<T> {
        let (sender, receiver) = channel();
        BlockingQueue {
            receiver: Arc::new(Mutex::new(receiver)),
            sender,
        }
    }


    /// #W3C says:
    /// Puts e last in the queue
    pub fn enqueue(&mut self, e: T) {
        self.sender.send(e).unwrap()
    }

    /// #W3C says:
    /// Removes and returns first element in queue, blocks if queue is empty
    pub fn dequeue(&mut self) -> T {
        self.receiver.lock().unwrap().recv().unwrap()
    }
}

/// #W3C says:
/// table[foo] returns the value associated with foo.
/// table[foo] = bar sets the value associated with foo to be bar.
/// #Actual implementation:
/// Instead of the Operators methoods are added.
#[derive(Debug)]
pub struct HashTable<K, T> {
    data: HashMap<K, T>,
}

impl<K: std::cmp::Eq + Hash + Clone, T: Clone> HashTable<K, T> {
    fn new() -> HashTable<K, T> {
        HashTable { data: HashMap::new() }
    }
    /// Extension to re-use exiting instances.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn put(&mut self, k: K, v: &T) {
        self.data.insert(k.clone(), v.clone());
    }

    pub fn putAll(&mut self, t: &HashTable<K, T>) {
        for (k, v) in &t.data {
            self.data.insert(k.clone(), v.clone());
        }
    }

    pub fn has(&self, k: K) -> bool {
        self.data.contains_key(&k)
    }


    pub fn get(&self, k: K) -> &T {
        self.data.get(&k).unwrap()
    }
}

/////////////////////////////////////////////////////////////
// FSM model (State etc, representing the XML-data-model)

pub type Name = String;
pub type StateMap = HashMap<StateId, State>;
pub type StateNameMap = HashMap<Name, StateId>;
pub type TransitionMap = HashMap<TransitionId, Transition>;

#[derive(Debug)]
#[derive(Clone, PartialEq, Copy)]
pub enum BindingType {
    Early,
    Late,
}

#[derive(Debug)]
pub struct Event {
    pub name: String,
    pub done_data: Option<DoneData>,
    pub invokeid: InvokeId,
}

impl Data for Event {
    fn get_copy(&self) -> Box<dyn Data> {
        Event::get_copy(self)
    }
}

impl Event {
    pub fn new(prefix: &str, state: StateId, data: &Option<DoneData>) -> Event {
        Event {
            name: format!("{}{}", prefix, state),
            done_data: { data.clone() },
            invokeid: 0,
        }
    }
    pub fn get_copy(&self) -> Box<Event> {
        Box::new(Event {
            invokeid: self.invokeid,
            done_data: self.done_data.clone(),
            name: self.name.clone(),
        })
    }
}

pub trait Tracer: Send + Debug {
    fn trace(&self, msg: &str);

    fn enterMethod(&self, what: &str) {
        self.trace(format!(">>> {}", what).as_str());
    }

    fn exitMethod(&self, what: &str) {
        self.trace(format!("<<< {}", what).as_str());
    }

    fn traceState(&self, what: &str, s: &State) {
        if s.name.is_empty() {
            self.trace(format!("{} #{}", what, s.id).as_str());
        } else {
            self.trace(format!("{} <{}>", what, &s.name).as_str());
        }
    }

    fn traceEnterState(&self, s: &State) {
        self.traceState("Enter", s);
    }

    fn traceExitState(&self, s: &State) {
        self.traceState("Exit", s);
    }

    fn traceEvent(&self, e: &Event) {
        self.trace(format!("Event <{}> #{}", &e.name, e.invokeid).as_str());
    }
}

#[derive(Debug)]
pub struct DefaultTracer {}

impl Tracer for DefaultTracer {
    fn trace(&self, msg: &str) {
        println!("{}", msg)
    }
}

impl DefaultTracer {
    pub fn new() -> DefaultTracer {
        DefaultTracer {}
    }
}

pub struct Fsm {
    pub configuration: OrderedSet<StateId>,
    pub statesToInvoke: OrderedSet<StateId>,
    pub datamodel: Box<dyn Datamodel>,
    pub internalQueue: Queue<Event>,
    pub externalQueue: BlockingQueue<Box<Event>>,
    pub historyValue: HashTable<StateId, OrderedSet<StateId>>,
    pub running: bool,
    pub binding: BindingType,
    pub version: String,
    pub tracer: Box<dyn Tracer>,

    /// A FSM can have actual multiple initial-target-states, so this state may be artificial.
    /// Reader have to generate a parent state if needed.
    /// This state also serve as the "scxml" state element were mentioned.
    pub pseudo_root: StateId,

    /**
     * The only real storage of states, identified by the Id
     * If a state has no declared id, one is generated.
     */
    pub states: StateMap,
    pub statesNames: StateNameMap,
    pub executableContent: HashMap<ExecutableContentId, ExecutableContent>,
    pub transitions: TransitionMap,

    pub data: DataStore,

}

impl Debug for Fsm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
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
        write!(f, "{}", *e.1)?;
    }

    write!(f, "}}")
}

fn display_transition_map(sm: &TransitionMap, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{{")?;

    let mut first = true;
    for e in sm {
        if first {
            first = false;
        } else {
            write!(f, ",")?;
        }
        write!(f, "{}", *e.1)?;
    }

    write!(f, "}}")
}


impl Fsm {
    pub fn new() -> Fsm {
        Fsm {
            configuration: OrderedSet::new(),
            version: "1.0".to_string(),
            datamodel: createDatamodel(ECMA_SCRIPT),
            internalQueue: Queue::new(),
            externalQueue: BlockingQueue::new(),
            historyValue: HashTable::new(),
            running: false,
            statesToInvoke: OrderedSet::new(),
            binding: BindingType::Early,
            states: HashMap::new(),
            statesNames: HashMap::new(),
            executableContent: HashMap::new(),
            transitions: HashMap::new(),
            data: DataStore::new(),
            pseudo_root: 0,
            tracer: Box::new(DefaultTracer::new()),
        }
    }

    pub fn get_state_by_name(&self, name: &Name) -> &State
    {
        self.get_state_by_id(*self.statesNames.get(name).unwrap())
    }

    pub fn get_state_by_name_mut(&mut self, name: &Name) -> &mut State
    {
        self.get_state_by_id_mut(*self.statesNames.get(name).unwrap())
    }


    /// Gets a state by id.
    /// The id MUST exists.
    pub fn get_state_by_id(&self, state_id: StateId) -> &State
    {
        self.states.get(&state_id).unwrap()
    }

    /// Gets a mutable state by id.
    /// The id MUST exists.
    pub fn get_state_by_id_mut(&mut self, state_id: StateId) -> &mut State
    {
        self.states.get_mut(&state_id).unwrap()
    }

    pub fn get_transition_by_id_mut(&mut self, transition_id: TransitionId) -> &mut Transition
    {
        self.transitions.get_mut(&transition_id).unwrap()
    }

    pub fn get_transition_by_id(&self, transition_id: TransitionId) -> &Transition
    {
        self.transitions.get(&transition_id).unwrap()
    }


    /// #W3C says:
    /// The purpose of this procedure is to initialize the interpreter and to start processing.
    ///
    /// In order to interpret an SCXML document, first (optionally) perform
    /// [xinclude](https://www.w3.org/TR/scxml/#xinclude) processing and (optionally) validate
    /// the document, throwing an exception if validation fails.
    /// Then convert initial attributes to \<initial\> container children with transitions
    /// to the state specified by the attribute. (This step is done purely to simplify the statement of
    /// the algorithm and has no effect on the system's behavior.
    ///
    /// Such transitions will not contain any executable content).
    /// Initialize the global data structures, including the data model.
    /// If binding is set to 'early', initialize the data model.
    /// Then execute the global \<script\> element, if any.
    /// Finally call enterStates on the initial configuration, set the global running
    /// variable to true and start the interpreter's event loop.
    /// ```
    /// procedure interpret(doc):
    ///     if not valid(doc): failWithError()
    ///     expandScxmlSource(doc)
    ///     configuration = new OrderedSet()
    ///     statesToInvoke = new OrderedSet()
    ///     internalQueue = new Queue()
    ///     externalQueue = new BlockingQueue()
    ///     historyValue = new HashTable()
    ///     datamodel = new Datamodel(doc)
    ///     if doc.binding == "early":
    ///         initializeDatamodel(datamodel, doc)
    ///     running = true
    ///     executeGlobalScriptElement(doc)
    ///     enterStates([doc.initial.transition])
    ///     mainEventLoop()
    /// ```
    /// #Actual implementation:
    pub fn interpret(&mut self) {
        if !self.valid() {
            self.failWithError()
        }
        self.expandScxmlSource();
        self.configuration = OrderedSet::new();
        self.statesToInvoke.clear();
        self.internalQueue.clear();
        self.historyValue.clear();
        self.datamodel.clear();
        if self.binding == BindingType::Early {
            self.datamodel.deref_mut().initializeDataModel(&self.data);
        }
        self.running = true;
        self.executeGlobalScriptElement();

        let mut initalStates = List::new();
        initalStates.push(self.get_state_by_id(self.pseudo_root).initial);
        self.enterStates(&initalStates);
        self.mainEventLoop();
    }


    /// #Actual implementation:
    /// This method should check if all state/transition references are correct.
    fn valid(&self) -> bool {
        true
    }

    /// #Actual implementation:
    /// Throws a panic
    fn failWithError(&self) {
        panic!("FSM has failed");
    }


    /// #Actual implementation:
    /// This method is called on the fsm model, after
    /// the xml document was processed. It should check if all References to states are fulfilled.
    /// After this method all "StateId" or "TransactionId" shall be valid and have to lead to a panic.
    fn expandScxmlSource(&mut self) {}

    fn executeGlobalScriptElement(&mut self) {}

    /// #W3C says:
    /// ## procedure mainEventLoop()
    /// This loop runs until we enter a top-level final state or an external entity cancels processing.
    /// In either case 'running' will be set to false (see EnterStates, below, for termination by
    /// entering a top-level final state.)
    ///
    /// At the top of the loop, we have either just entered the state machine, or we have just
    /// processed an external event. Each iteration through the loop consists of four main steps:
    /// 1) Complete the macrostep by repeatedly taking any internally enabled transitions, namely
    /// those that don't require an event or that are triggered by an internal event.
    /// After each such transition/microstep, check to see if we have reached a final state.
    /// 2) When there are no more internally enabled transitions available, the macrostep is done.
    /// Execute any \<invoke\> tags for states that we entered on the last iteration through the loop
    /// 3) If any internal events have been generated by the invokes, repeat step 1 to handle any
    /// errors raised by the \<invoke\> elements.
    /// 4) When the internal event queue is empty, wait for
    /// an external event and then execute any transitions that it triggers. However special
    /// preliminary processing is applied to the event if the state has executed any \<invoke\>
    /// elements. First, if this event was generated by an invoked process, apply \<finalize\>
    /// processing to it. Secondly, if any \<invoke\> elements have autoforwarding set, forward the
    /// event to them. These steps apply before the transitions are taken.
    ///
    /// This event loop thus enforces run-to-completion semantics, in which the system process an external event and then takes all the 'follow-up' transitions that the processing has enabled before looking for another external event. For example, suppose that the external event queue contains events ext1 and ext2 and the machine is in state s1. If processing ext1 takes the machine to s2 and generates internal event int1, and s2 contains a transition t triggered by int1, the system is guaranteed to take t, no matter what transitions s2 or other states have that would be triggered by ext2. Note that this is true even though ext2 was already in the external event queue when int1 was generated. In effect, the algorithm treats the processing of int1 as finishing up the processing of ext1.
    /// ```
    /// procedure mainEventLoop():
    ///     while running:
    ///         enabledTransitions = null
    ///         macrostepDone = false
    ///         # Here we handle eventless transitions and transitions
    ///         # triggered by internal events until macrostep is complete
    ///         while running and not macrostepDone:
    ///             enabledTransitions = selectEventlessTransitions()
    ///             if enabledTransitions.isEmpty():
    ///                 if internalQueue.isEmpty():
    ///                     macrostepDone = true
    ///                 else:
    ///                     internalEvent = internalQueue.dequeue()
    ///                     datamodel["_event"] = internalEvent
    ///                     enabledTransitions = selectTransitions(internalEvent)
    ///             if not enabledTransitions.isEmpty():
    ///                 microstep(enabledTransitions.toList())
    ///         # either we're in a final state, and we break out of the loop
    ///         if not running:
    ///             break
    ///         # or we've completed a macrostep, so we start a new macrostep by waiting for an external event
    ///         # Here we invoke whatever needs to be invoked. The implementation of 'invoke' is platform-specific
    ///         for state in statesToInvoke.sort(entryOrder):
    ///             for inv in state.invoke.sort(documentOrder):
    ///                 invoke(inv)
    ///         statesToInvoke.clear()
    ///         # Invoking may have raised internal error events and we iterate to handle them
    ///         if not internalQueue.isEmpty():
    ///             continue
    ///         # A blocking wait for an external event.  Alternatively, if we have been invoked
    ///         # our parent session also might cancel us.  The mechanism for this is platform specific,
    ///         # but here we assume it’s a special event we receive
    ///         externalEvent = externalQueue.dequeue()
    ///         if isCancelEvent(externalEvent):
    ///             running = false
    ///             continue
    ///         datamodel["_event"] = externalEvent
    ///         for state in configuration:
    ///             for inv in state.invoke:
    ///                 if inv.invokeid == externalEvent.invokeid:
    ///                     applyFinalize(inv, externalEvent)
    ///                 if inv.autoforward:
    ///                     send(inv.id, externalEvent)
    ///         enabledTransitions = selectTransitions(externalEvent)
    ///         if not enabledTransitions.isEmpty():
    ///             microstep(enabledTransitions.toList())
    ///     # End of outer while running loop.  If we get here, we have reached a top-level final state or have been cancelled
    ///     exitInterpreter()
    /// ```
    /// #Actual implementation:
    ///  todo
    fn mainEventLoop(&mut self) {
        self.tracer.enterMethod("mainEventLoop");
        while self.running {
            let mut enabledTransitions;
            let mut macrostepDone = false;
            // Here we handle eventless transitions and transitions
            // triggered by internal events until macrostep is complete
            while self.running && !macrostepDone {
                enabledTransitions = self.selectEventlessTransitions();
                if enabledTransitions.isEmpty() {
                    if self.internalQueue.isEmpty() {
                        macrostepDone = true;
                    } else {
                        self.tracer.enterMethod("internalQueue.dequeue");
                        let internalEvent = self.internalQueue.dequeue();
                        self.tracer.exitMethod("internalQueue.dequeue");
                        self.datamodel.set(&"_event".to_string(), internalEvent.get_copy());
                        enabledTransitions = self.selectTransitions(&internalEvent);
                    }
                }
                if !enabledTransitions.isEmpty() {
                    self.microstep(&enabledTransitions.toList())
                }
            }
            // either we're in a final state, and we break out of the loop
            if !self.running {
                break;
            }
            // or we've completed a macrostep, so we start a new macrostep by waiting for an external event
            // Here we invoke whatever needs to be invoked. The implementation of 'invoke' is platform-specific
            for sid in self.statesToInvoke.sort(&entryOrder).iterator() {
                let state = self.get_state_by_id(*sid);
                for inv in state.invoke.sort(&invokeDocumentOrder).iterator() {
                    self.invoke(inv);
                }
            }
            self.statesToInvoke.clear();
            // Invoking may have raised internal error events and we iterate to handle them
            if !self.internalQueue.isEmpty() {
                continue;
            }
            // A blocking wait for an external event.  Alternatively, if we have been invoked
            // our parent session also might cancel us.  The mechanism for this is platform specific,
            // but here we assume it’s a special event we receive
            self.tracer.enterMethod("externalQueue.dequeue");
            let externalEvent = self.externalQueue.dequeue();
            self.tracer.exitMethod("externalQueue.dequeue");
            if self.isCancelEvent(&externalEvent) {
                self.running = false;
                continue;
            }
            let mut toFinalize: Vec<InvokeId> = Vec::new();
            let mut toForward: Vec<InvokeId> = Vec::new();
            {
                let invokeId = externalEvent.invokeid;
                self.datamodel.set(&"_event".to_string(), externalEvent.get_copy());
                for sid in self.configuration.iterator() {
                    let state = self.get_state_by_id(*sid);
                    for inv in state.invoke.iterator() {
                        let mut sendIt = false;
                        if inv.invokeid == invokeId {
                            toFinalize.push(inv.id);
                        }
                        if inv.autoforward {
                            toForward.push(inv.id);
                        }
                    }
                }
            }
            for invId in toFinalize {
                self.applyFinalize(invId, &externalEvent);
            }
            for invId in toForward {
                self.send(invId, &externalEvent);
            }

            enabledTransitions = self.selectTransitions(&externalEvent);
            if !enabledTransitions.isEmpty() {
                self.microstep(&enabledTransitions.toList());
            }
        }
        // End of outer while running loop.  If we get here, we have reached a top-level final state or have been cancelled
        self.exitInterpreter();
        self.tracer.exitMethod("mainEventLoop");
    }

    /// #W3C says:
    /// # procedure exitInterpreter()
    /// The purpose of this procedure is to exit the current SCXML process by exiting all active
    /// states. If the machine is in a top-level final state, a Done event is generated.
    /// (Note that in this case, the final state will be the only active state.)
    /// The implementation of returnDoneEvent is platform-dependent, but if this session is the
    /// result of an \<invoke\> in another SCXML session, returnDoneEvent will cause the event
    /// done.invoke.\<id\> to be placed in the external event queue of that session, where \<id\> is
    /// the id generated in that session when the \<invoke\> was executed.
    /// ```
    /// procedure exitInterpreter():
    ///     statesToExit = configuration.toList().sort(exitOrder)
    ///     for s in statesToExit:
    ///         for content in s.onexit.sort(documentOrder):
    ///             executeContent(content)
    ///         for inv in s.invoke:
    ///             cancelInvoke(inv)
    ///         configuration.delete(s)
    ///         if isFinalState(s) and isScxmlElement(s.parent):
    ///             returnDoneEvent(s.donedata)
    /// ```
    /// #Actual implementation:
    ///  todo
    fn exitInterpreter(&mut self) {
        todo!()
    }


    /// #W3C says:
    /// # function selectEventlessTransitions()
    /// This function selects all transitions that are enabled in the current configuration that
    /// do not require an event trigger. First find a transition with no 'event' attribute whose
    /// condition evaluates to true. If multiple matching transitions are present, take the first
    /// in document order. If none are present, search in the state's ancestors in ancestry order
    /// until one is found. As soon as such a transition is found, add it to enabledTransitions,
    /// and proceed to the next atomic state in the configuration. If no such transition is found
    /// in the state or its ancestors, proceed to the next state in the configuration.
    /// When all atomic states have been visited and transitions selected, filter the set of enabled
    /// transitions, removing any that are preempted by other transitions, then return the
    /// resulting set.
    /// ```
    ///
    /// function selectEventlessTransitions():
    ///     enabledTransitions = new OrderedSet()
    ///     atomicStates = configuration.toList().filter(isAtomicState).sort(documentOrder)
    ///     for state in atomicStates:
    ///         loop: for s in [state].append(getProperAncestors(state, null)):
    ///             for t in s.transition.sort(documentOrder):
    ///                 if not t.event and conditionMatch(t):
    ///                     enabledTransitions.add(t)
    ///                     break loop
    ///     enabledTransitions = removeConflictingTransitions(enabledTransitions)
    ///     return enabledTransitions
    /// ```
    /// #Actual implementation:
    fn selectEventlessTransitions(&mut self) -> OrderedSet<TransitionId> {
        self.tracer.enterMethod("selectEventlessTransitions");

        let mut enabledTransitions: OrderedSet<TransitionId> = OrderedSet::new();
        let atomicStates = self.configuration.toList().filterBy(&|sid| -> bool { self.isAtomicState(*sid) }).sort(&documentOrder);
        for state in atomicStates.iterator() {
            let mut states: List<StateId> = List::new();
            states.push(*state);
            states.appendSet(&self.getProperAncestors(*state, 0));
            'outer: for s in states.iterator() {
                let state = self.get_state_by_id(*s);
                for tid in state.transitions.sort(&documentOrder).iterator() {
                    let t = self.get_transition_by_id(*tid);
                    if !t.events.is_empty() && self.conditionMatch(t) {
                        enabledTransitions.add(*tid);
                        break 'outer;
                    }
                }
            }
            enabledTransitions = self.removeConflictingTransitions(&enabledTransitions);
        }
        self.tracer.exitMethod("selectEventlessTransitions");
        enabledTransitions
    }

    /// #W3C says:
    /// function selectTransitions(event)
    /// The purpose of the selectTransitions()procedure is to collect the transitions that are enabled by this event in the current configuration.
    ///
    /// Create an empty set of enabledTransitions. For each atomic state , find a transition whose 'event' attribute matches event and whose condition evaluates to true. If multiple matching transitions are present, take the first in document order. If none are present, search in the state's ancestors in ancestry order until one is found. As soon as such a transition is found, add it to enabledTransitions, and proceed to the next atomic state in the configuration. If no such transition is found in the state or its ancestors, proceed to the next state in the configuration. When all atomic states have been visited and transitions selected, filter out any preempted transitions and return the resulting set.
    /// ```
    /// function selectTransitions(event):
    ///     enabledTransitions = new OrderedSet()
    ///     atomicStates = configuration.toList().filter(isAtomicState).sort(documentOrder)
    ///     for state in atomicStates:
    ///         loop: for s in [state].append(getProperAncestors(state, null)):
    ///             for t in s.transition.sort(documentOrder):
    ///                 if t.event and nameMatch(t.event, event.name) and conditionMatch(t):
    ///                     enabledTransitions.add(t)
    ///                     break loop
    ///     enabledTransitions = removeConflictingTransitions(enabledTransitions)
    ///     return enabledTransitions
    /// ```
    /// #Actual implementation:
    ///  todo (check argument types!)
    fn selectTransitions(&mut self, event: &Event) -> OrderedSet<TransitionId> {
        self.tracer.enterMethod("selectTransitions");
        self.tracer.trace(format!("->Event: {}", &event.name).as_str());
        let mut enabledTransitions: OrderedSet<TransitionId> = OrderedSet::new();
        let atomicStates = self.configuration.toList().filterBy(&|sid| -> bool { self.isAtomicState(*sid) }).sort(&documentOrder);
        for state in atomicStates.iterator() {
            'outer:
            for sid in List::from_array(&[*state]).appendSet(&self.getProperAncestors(*state, 0)).iterator() {
                let s = self.get_state_by_id(*sid);
                let mut transition: Vec<&Transition> = Vec::new();
                for tid in s.transitions.iterator() {
                    transition.push(self.get_transition_by_id(*tid));
                }

                transition.sort_by(&transitionDocumentOrder);
                for t in transition {
                    if (!t.events.is_empty()) && self.nameMatch(&t.events, &event.name) && self.conditionMatch(t)
                    {
                        self.tracer.trace(format!(" Matching Transition: {} -> {:?}", t.id, &t.target).as_str());
                        enabledTransitions.add(t.id);
                        break 'outer;
                    }
                    enabledTransitions = self.removeConflictingTransitions(&enabledTransitions);
                }
            }
        }
        self.tracer.exitMethod("selectTransitions");
        enabledTransitions
    }


    /// #W3C says:
    /// #function removeConflictingTransitions(enabledTransitions)
    /// enabledTransitions will contain multiple transitions only if a parallel state is active.
    /// In that case, we may have one transition selected for each of its children.
    /// These transitions may conflict with each other in the sense that they have incompatible
    /// target states. Loosely speaking, transitions are compatible when each one is contained
    /// within a single \<state\> child of the \<parallel\> element.
    /// Transitions that aren't contained within a single child force the state
    /// machine to leave the \<parallel\> ancestor (even if they reenter it later). Such transitions
    /// conflict with each other, and with transitions that remain within a single <state> child, in that
    /// they may have targets that cannot be simultaneously active. The test that transitions have non-
    /// intersecting exit sets captures this requirement. (If the intersection is null, the source and
    /// targets of the two transitions are contained in separate <state> descendants of \<parallel\>.
    /// If intersection is non-null, then at least one of the transitions is exiting the \<parallel\>).
    /// When such a conflict occurs, then if the source state of one of the transitions is a descendant
    /// of the source state of the other, we select the transition in the descendant. Otherwise we prefer
    /// the transition that was selected by the earlier state in document order and discard the other
    /// transition. Note that targetless transitions have empty exit sets and thus do not conflict with
    /// any other transitions.
    ///
    /// We start with a list of enabledTransitions and produce a conflict-free list of filteredTransitions. For each t1 in enabledTransitions, we test it against all t2 that are already selected in filteredTransitions. If there is a conflict, then if t1's source state is a descendant of t2's source state, we prefer t1 and say that it preempts t2 (so we we make a note to remove t2 from filteredTransitions). Otherwise, we prefer t2 since it was selected in an earlier state in document order, so we say that it preempts t1. (There's no need to do anything in this case since t2 is already in filteredTransitions. Furthermore, once one transition preempts t1, there is no need to test t1 against any other transitions.) Finally, if t1 isn't preempted by any transition in filteredTransitions, remove any transitions that it preempts and add it to that list.
    /// ```
    /// function removeConflictingTransitions(enabledTransitions):
    ///     filteredTransitions = new OrderedSet()
    ///     //toList sorts the transitions in the order of the states that selected them
    ///     for t1 in enabledTransitions.toList():
    ///         t1Preempted = false
    ///         transitionsToRemove = new OrderedSet()
    ///         for t2 in filteredTransitions.toList():
    ///             if computeExitSet([t1]).hasIntersection(computeExitSet([t2])):
    ///                 if isDescendant(t1.source, t2.source):
    ///                     transitionsToRemove.add(t2)
    ///                 else:
    ///                     t1Preempted = true
    ///                     break
    ///         if not t1Preempted:
    ///             for t3 in transitionsToRemove.toList():
    ///                 filteredTransitions.delete(t3)
    ///             filteredTransitions.add(t1)
    ///
    ///     return filteredTransitions
    /// ```
    /// #Actual implementation:
    ///  todo (check argument types!)
    fn removeConflictingTransitions(&self, enabledTransitions: &OrderedSet<TransitionId>) -> OrderedSet<TransitionId> {
        let mut filteredTransitions: OrderedSet<TransitionId> = OrderedSet::new();
        //toList sorts the transitions in the order of the states that selected them
        for tid1 in enabledTransitions.toList().iterator() {
            let t1 = self.get_transition_by_id(*tid1);
            let mut t1Preempted = false;
            let mut transitionsToRemove = OrderedSet::new();
            let filteredTransitionList = filteredTransitions.toList();
            for tid2 in filteredTransitionList.iterator()
            {
                let t2 = self.get_transition_by_id(*tid2);
                if self.computeExitSet(&List::from_array(&[*tid1])).hasIntersection(&self.computeExitSet(&List::from_array(&[*tid2]))) {
                    let t2obj = self.get_transition_by_id(*tid2);
                    if self.isDescendant(t1.source, t2.source) {
                        transitionsToRemove.add(tid2);
                    } else {
                        t1Preempted = true;
                        break;
                    }
                }
            }
            if !t1Preempted {
                for t3 in transitionsToRemove.toList().iterator() {
                    filteredTransitions.delete(t3);
                }
                filteredTransitions.add(*tid1);
            }
        }
        filteredTransitions
    }


    /// #W3C says:
    /// # procedure microstep(enabledTransitions)
    /// The purpose of the microstep procedure is to process a single set of transitions. These may have been enabled by an external event, an internal event, or by the presence or absence of certain values in the data model at the current point in time. The processing of the enabled transitions must be done in parallel ('lock step') in the sense that their source states must first be exited, then their actions must be executed, and finally their target states entered.
    ///
    /// If a single atomic state is active, then enabledTransitions will contain only a single transition. If multiple states are active (i.e., we are in a parallel region), then there may be multiple transitions, one per active atomic state (though some states may not select a transition.) In this case, the transitions are taken in the document order of the atomic states that selected them.
    /// ```
    /// procedure microstep(enabledTransitions):
    ///     exitStates(enabledTransitions)
    ///     executeTransitionContent(enabledTransitions)
    ///     enterStates(enabledTransitions)
    /// ```
    /// #Actual implementation:
    fn microstep(&mut self, enabledTransitions: &List<TransitionId>) {
        self.tracer.enterMethod("microstep");
        self.exitStates(enabledTransitions);
        self.executeTransitionContent(enabledTransitions);
        self.enterStates(enabledTransitions);
        self.tracer.exitMethod("microstep");
    }


    /// #W3C says:
    /// # procedure exitStates(enabledTransitions)
    /// Compute the set of states to exit. Then remove all the states on statesToExit from the set of states that will have invoke processing done at the start of the next macrostep. (Suppose macrostep M1 consists of microsteps m11 and m12. We may enter state s in m11 and exit it in m12. We will add s to statesToInvoke in m11, and must remove it in m12. In the subsequent macrostep M2, we will apply invoke processing to all states that were entered, and not exited, in M1.) Then convert statesToExit to a list and sort it in exitOrder.
    ///
    /// For each state s in the list, if s has a deep history state h, set the history value of h to be the list of all atomic descendants of s that are members in the current configuration, else set its value to be the list of all immediate children of s that are members of the current configuration. Again for each state s in the list, first execute any onexit handlers, then cancel any ongoing invocations, and finally remove s from the current configuration.
    ///
    /// ```
    /// procedure exitStates(enabledTransitions):
    ///     statesToExit = computeExitSet(enabledTransitions)
    ///     for s in statesToExit:
    ///         statesToInvoke.delete(s)
    ///     statesToExit = statesToExit.toList().sort(exitOrder)
    ///     for s in statesToExit:
    ///         for h in s.history:
    ///             if h.type == "deep":
    ///                 f = lambda s0: isAtomicState(s0) and isDescendant(s0,s)
    ///             else:
    ///                 f = lambda s0: s0.parent == s
    ///             historyValue[h.id] = configuration.toList().filter(f)
    ///     for s in statesToExit:
    ///         for content in s.onexit.sort(documentOrder):
    ///             executeContent(content)
    ///         for inv in s.invoke:
    ///             cancelInvoke(inv)
    ///         configuration.delete(s)
    /// ```
    /// #Actual implementation:
    ///  todo (check argument types!)
    fn exitStates(&mut self, enabledTransitions: &List<TransitionId>) {
        self.tracer.enterMethod("exitStates");

        let statesToExit = self.computeExitSet(enabledTransitions);
        for s in statesToExit.iterator() {
            self.statesToInvoke.delete(s);
        }
        let statesToExitSorted = statesToExit.toList().sort(&exitOrder);
        let mut ahistory: HashTable<StateId, OrderedSet<StateId>> = HashTable::new();
        for sid in statesToExitSorted.iterator() {
            let s = self.get_state_by_id(*sid);
            for hid in s.history.iterator() {
                let h = self.get_state_by_id(*hid);
                if h.history_type == HistoryType::Deep
                {
                    ahistory.put(h.id, &self.configuration.toList().filterBy(
                        &|s0| -> bool { self.isAtomicState(*s0) && self.isDescendant(*s0, *sid) }).toSet());
                } else {
                    ahistory.put(h.id, &self.configuration.toList().filterBy(
                        &|s0| -> bool { self.get_state_by_id(*s0).parent == *sid }).toSet());
                }
            }
        }
        self.historyValue.putAll(&ahistory);
        for sid in statesToExitSorted.iterator() {
            let exe: List<ExecutableContentId> = List::new();
            {
                let s = self.get_state_by_id(*sid);
                exe.append(&s.onexit.sort(&documentOrder));
            }

            for content in exe.iterator() {
                self.executeContent(*content);
            }

            let mut invokeList: List<InvokeId> = List::new();
            {
                let s = self.get_state_by_id(*sid);
                for inv in s.invoke.iterator() {
                    invokeList.push(inv.invokeid);
                }
            }

            for invokeId in invokeList.iterator() {
                self.cancelInvoke(*invokeId);
            }

            self.configuration.delete(sid)
        }
        self.tracer.exitMethod("exitStates");
    }


    /// #W3C says:
    /// ## procedure enterStates(enabledTransitions)
    /// First, compute the list of all the states that will be entered as a result of taking the
    /// transitions in enabledTransitions. Add them to statesToInvoke so that invoke processing can
    /// be done at the start of the next macrostep. Convert statesToEnter to a list and sort it in
    /// entryOrder. For each state s in the list, first add s to the current configuration.
    /// Then if we are using late binding, and this is the first time we have entered s, initialize
    /// its data model. Then execute any onentry handlers. If s's initial state is being entered by
    /// default, execute any executable content in the initial transition. If a history state in s
    /// was the target of a transition, and s has not been entered before, execute the content
    /// inside the history state's default transition. Finally, if s is a final state, generate
    /// relevant Done events. If we have reached a top-level final state, set running to false as a
    /// signal to stop processing.
    /// ```
    ///    procedure enterStates(enabledTransitions):
    ///        statesToEnter = new OrderedSet()
    ///        statesForDefaultEntry = new OrderedSet()
    ///        // initialize the temporary table for default content in history states
    ///        defaultHistoryContent = new HashTable()
    ///        computeEntrySet(enabledTransitions, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    ///        for s in statesToEnter.toList().sort(entryOrder):
    ///           configuration.add(s)
    ///           statesToInvoke.add(s)
    ///           if binding == "late" and s.isFirstEntry:
    ///              initializeDataModel(datamodel.s,doc.s)
    ///              s.isFirstEntry = false
    ///           for content in s.onentry.sort(documentOrder):
    ///              executeContent(content)
    ///           if statesForDefaultEntry.isMember(s):
    ///              executeContent(s.initial.transition)
    ///           if defaultHistoryContent[s.id]:
    ///              executeContent(defaultHistoryContent[s.id])
    ///           if isFinalState(s):
    ///              if isSCXMLElement(s.parent):
    ///                 running = false
    ///              else:
    ///                 parent = s.parent
    ///                 grandparent = parent.parent
    ///                 internalQueue.enqueue(new Event("done.state." + parent.id, s.donedata))
    ///                 if isParallelState(grandparent):
    ///                    if getChildStates(grandparent).every(isInFinalState):
    ///                       internalQueue.enqueue(new Event("done.state." + grandparent.id))
    /// ```
    /// #Actual implementation:
    ///
    fn enterStates(&mut self, enabledTransitions: &List<StateId>) {
        let binding = self.binding;
        let mut statesToEnter = OrderedSet::new();
        let mut statesForDefaultEntry = OrderedSet::new();

        // initialize the temporary table for default content in history states
        let mut defaultHistoryContent: HashTable<StateId, ExecutableContentId> = HashTable::new();
        self.computeEntrySet(enabledTransitions, &mut statesToEnter, &mut statesForDefaultEntry, &mut defaultHistoryContent);
        for s in statesToEnter.toList().sort(&entryOrder).iterator() {
            {
                self.tracer.traceEnterState(&self.get_state_by_id(*s));
            }
            self.configuration.add(*s);
            self.statesToInvoke.add(*s);
            let stateS: &mut State = self.get_state_by_id_mut(*s);
            if binding == BindingType::Late && stateS.isFirstEntry {
                stateS.datamodel.initializeDataModel(&stateS.data);
                stateS.isFirstEntry = false;
            }
            for content in stateS.onentry.sort(&documentOrder).iterator() {
                self.executeContent(*content);
            }
            if statesForDefaultEntry.isMember(&s) {
                let stateS: &State = self.get_state_by_id(*s);
                if stateS.initial > 0 {
                    self.executeContent(self.get_transition_by_id(stateS.initial).content);
                }
            }
            if defaultHistoryContent.has(*s) {
                self.executeContent(*defaultHistoryContent.get(*s));
            }
            if self.isFinalState(*s) {
                let stateS = self.get_state_by_id(*s);
                let parent: StateId = stateS.parent;
                if self.isSCXMLElement(parent) {
                    self.running = false
                } else {
                    self.enqueue_internal(Event::new("done.state.", parent, &stateS.donedata));
                    let stateParent = self.get_state_by_id(parent);
                    let grandparent: StateId = stateParent.parent;
                    if self.isParallelState(grandparent) {
                        if self.getChildStates(grandparent).every(
                            &|s: &StateId| -> bool{ self.isInFinalState(*s) }) {
                            self.enqueue_internal(Event::new("done.state.", grandparent, &None));
                        }
                    }
                }
            }
        }
    }

    /// Put an event into the internal queue.
    pub fn enqueue_internal(&mut self, event: Event) {
        self.internalQueue.enqueue(event);
    }

    pub fn executeContent(&self, contentId: ExecutableContentId) {
        // TODO
    }

    pub fn isParallelState(&self, state: StateId) -> bool {
        state > 0 && self.get_state_by_id(state).is_parallel
    }

    pub fn isSCXMLElement(&self, state: StateId) -> bool {
        state == self.pseudo_root
    }

    pub fn isFinalState(&self, state: StateId) -> bool {
        self.get_state_by_id(state).is_final
    }

    /// State which any sub-state is called "atomic"
    pub fn isAtomicState(&self, state: StateId) -> bool {
        self.get_state_by_id(state).states.is_empty()
    }


    /// #W3C says:
    /// # procedure computeExitSet(enabledTransitions)
    /// For each transition t in enabledTransitions, if t is targetless then do nothing, else compute the transition's domain.
    /// (This will be the source state in the case of internal transitions) or the least common compound ancestor
    /// state of the source state and target states of t (in the case of external transitions. Add to the statesToExit
    /// set all states in the configuration that are descendants of the domain.
    /// ```
    /// function computeExitSet(transitions)
    ///     statesToExit = new OrderedSet
    ///     for t in transitions:
    ///         if t.target:
    ///             domain = getTransitionDomain(t)
    ///             for s in configuration:
    ///                 if isDescendant(s,domain):
    ///                     statesToExit.add(s)
    ///     return statesToExit
    /// ```
    fn computeExitSet(&self, transitions: &List<TransitionId>) -> OrderedSet<StateId> {
        let mut statesToExit: OrderedSet<StateId> = OrderedSet::new();
        for tid in transitions.iterator() {
            let t = self.get_transition_by_id(*tid);
            if !t.target.is_empty() {
                let domain = self.getTransitionDomain(t);
                for s in self.configuration.iterator() {
                    if self.isDescendant(*s, domain) {
                        statesToExit.add(*s);
                    }
                }
            }
        }
        statesToExit
    }

    /// #W3C says:
    /// # procedure executeTransitionContent(enabledTransitions)
    /// For each transition in the list of enabledTransitions, execute its executable content.
    /// ```
    /// procedure executeTransitionContent(enabledTransitions):
    ///     for t in enabledTransitions:
    ///         executeContent(t)
    /// ```
    fn executeTransitionContent(&mut self, enabledTransitions: &List<TransitionId>) {
        for tid in enabledTransitions.iterator() {
            let t = self.get_transition_by_id(*tid);
            self.executeContent(t.content);
        }
    }

    /// #W3C says:
    /// # procedure computeEntrySet(transitions, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    /// Compute the complete set of states that will be entered as a result of taking 'transitions'.
    /// This value will be returned in 'statesToEnter' (which is modified by this procedure). Also
    /// place in 'statesForDefaultEntry' the set of all states whose default initial states were
    /// entered. First gather up all the target states in 'transitions'. Then add them and, for all
    /// that are not atomic states, add all of their (default) descendants until we reach one or
    /// more atomic states. Then add any ancestors that will be entered within the domain of the
    /// transition. (Ancestors outside of the domain of the transition will not have been exited.)
    /// ```
    /// procedure computeEntrySet(transitions, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    ///     for t in transitions:
    ///         for s in t.target:
    ///             addDescendantStatesToEnter(s,statesToEnter,statesForDefaultEntry, defaultHistoryContent)
    ///         ancestor = getTransitionDomain(t)
    ///         for s in getEffectiveTargetStates(t)):
    ///             addAncestorStatesToEnter(s, ancestor, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    /// ```
    /// #Actual implementation:
    ///  todo (check argument types!)
    fn computeEntrySet(&mut self, transitions: &List<TransitionId>, statesToEnter: &mut OrderedSet<StateId>,
                       statesForDefaultEntry: &mut OrderedSet<StateId>, defaultHistoryContent: &mut HashTable<StateId, ExecutableContentId>) {
        for tid in transitions.iterator() {
            let t = self.get_transition_by_id(*tid);
            for s in t.target.iter() {
                self.addDescendantStatesToEnter(*s, statesToEnter, statesForDefaultEntry, defaultHistoryContent);
            }
            let ancestor = self.getTransitionDomain(t);
            for s in self.getEffectiveTargetStates(t).iterator() {
                self.addAncestorStatesToEnter(*s, ancestor, statesToEnter, statesForDefaultEntry, defaultHistoryContent);
            }
        }
    }

    /// #W3C says:
    /// #procedure addDescendantStatesToEnter(state,statesToEnter,statesForDefaultEntry, defaultHistoryContent)
    /// The purpose of this procedure is to add to statesToEnter 'state' and any of its descendants
    /// that the state machine will end up entering when it enters 'state'. (N.B. If 'state' is a
    /// history pseudo-state, we dereference it and add the history value instead.) Note that this '
    /// procedure permanently modifies both statesToEnter and statesForDefaultEntry.
    ///
    /// First, If state is a history state then add either the history values associated with state or state's default target to statesToEnter. Then (since the history value may not be an immediate descendant of 'state's parent) add any ancestors between the history value and state's parent. Else (if state is not a history state), add state to statesToEnter. Then if state is a compound state, add state to statesForDefaultEntry and recursively call addStatesToEnter on its default initial state(s). Then, since the default initial states may not be children of 'state', add any ancestors between the default initial states and 'state'. Otherwise, if state is a parallel state, recursively call addStatesToEnter on any of its child states that don't already have a descendant on statesToEnter.
    /// ```
    /// procedure addDescendantStatesToEnter(state,statesToEnter,statesForDefaultEntry, defaultHistoryContent):
    ///     if isHistoryState(state):
    ///         if historyValue[state.id]:
    ///             for s in historyValue[state.id]:
    ///                 addDescendantStatesToEnter(s,statesToEnter,statesForDefaultEntry, defaultHistoryContent)
    ///             for s in historyValue[state.id]:
    ///                 addAncestorStatesToEnter(s, state.parent, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    ///         else:
    ///             defaultHistoryContent[state.parent.id] = state.transition.content
    ///             for s in state.transition.target:
    ///                 addDescendantStatesToEnter(s,statesToEnter,statesForDefaultEntry, defaultHistoryContent)
    ///             for s in state.transition.target:
    ///                 addAncestorStatesToEnter(s, state.parent, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    ///     else:
    ///         statesToEnter.add(state)
    ///         if isCompoundState(state):
    ///             statesForDefaultEntry.add(state)
    ///             for s in state.initial.transition.target:
    ///                 addDescendantStatesToEnter(s,statesToEnter,statesForDefaultEntry, defaultHistoryContent)
    ///             for s in state.initial.transition.target:
    ///                 addAncestorStatesToEnter(s, state, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    ///         else:
    ///             if isParallelState(state):
    ///                 for child in getChildStates(state):
    ///                     if not statesToEnter.some(lambda s: isDescendant(s,child)):
    ///                         addDescendantStatesToEnter(child,statesToEnter,statesForDefaultEntry, defaultHistoryContent)
    /// ```
    fn addDescendantStatesToEnter(&self, sid: StateId, statesToEnter: &mut OrderedSet<StateId>,
                                  statesForDefaultEntry: &mut OrderedSet<StateId>, defaultHistoryContent: &mut HashTable<StateId, ExecutableContentId>) {
        let state = self.get_state_by_id(sid);
        if self.isHistoryState(sid) {
            if self.historyValue.has(sid) {
                for s in self.historyValue.get(state.id).iterator()
                {
                    self.addDescendantStatesToEnter(sid, statesToEnter, statesForDefaultEntry, defaultHistoryContent);
                }
                for s in self.historyValue.get(sid).iterator() {
                    self.addAncestorStatesToEnter(sid, state.parent, statesToEnter, statesForDefaultEntry, defaultHistoryContent);
                }
            } else {
                // A history state have exactly one transition which specified the default history configuration.
                let defaultTransition = self.get_transition_by_id(*state.transitions.head());
                defaultHistoryContent.put(state.parent, &defaultTransition.content);
                for s in &defaultTransition.target {
                    self.addDescendantStatesToEnter(*s, statesToEnter, statesForDefaultEntry, defaultHistoryContent);
                }
                for s in &defaultTransition.target {
                    self.addAncestorStatesToEnter(*s, state.parent, statesToEnter, statesForDefaultEntry, defaultHistoryContent);
                }
            }
        } else {
            statesToEnter.add(sid);
            if self.isCompoundState(sid) {
                statesForDefaultEntry.add(sid);
                if state.initial != 0 {
                    let initialTransition = self.get_transition_by_id(state.initial);
                    for s in &initialTransition.target {
                        self.addDescendantStatesToEnter(*s, statesToEnter, statesForDefaultEntry, defaultHistoryContent);
                    }
                    for s in &initialTransition.target {
                        self.addAncestorStatesToEnter(*s, sid, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
                    }
                }
            } else {
                if self.isParallelState(sid) {
                    for child in self.getChildStates(sid).iterator() {
                        if !statesToEnter.some(&|s| { self.isDescendant(*s, *child) }) {
                            self.addDescendantStatesToEnter(*child, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
                        }
                    }
                }
            }
        }
    }

    /// #W3C says:
    /// # procedure addAncestorStatesToEnter(state, ancestor, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    /// Add to statesToEnter any ancestors of 'state' up to, but not including, 'ancestor' that must be entered in order to enter 'state'. If any of these ancestor states is a parallel state, we must fill in its descendants as well.
    /// ```
    /// procedure addAncestorStatesToEnter(state, ancestor, statesToEnter, statesForDefaultEntry, defaultHistoryContent)
    ///     for anc in getProperAncestors(state,ancestor):
    ///         statesToEnter.add(anc)
    ///         if isParallelState(anc):
    ///             for child in getChildStates(anc):
    ///                 if not statesToEnter.some(lambda s: isDescendant(s,child)):
    ///                     addDescendantStatesToEnter(child,statesToEnter,statesForDefaultEntry, defaultHistoryContent)
    /// ```
    /// #Actual implementation:
    fn addAncestorStatesToEnter(&self, state: StateId, ancestor: StateId, statesToEnter: &mut OrderedSet<StateId>,
                                statesForDefaultEntry: &mut OrderedSet<StateId>, defaultHistoryContent: &mut HashTable<StateId, ExecutableContentId>) {
        for anc in self.getProperAncestors(state, ancestor).iterator() {
            statesToEnter.add(*anc);
            if self.isParallelState(*anc) {
                for child in self.getChildStates(*anc).iterator() {
                    if !statesToEnter.some(&|s| {
                        self.isDescendant(*s, *child)
                    }) {
                        self.addDescendantStatesToEnter(*child, statesToEnter, statesForDefaultEntry, defaultHistoryContent);
                    }
                }
            }
        }
    }

    /// #W3C says:
    /// # procedure isInFinalState(s)
    /// Return true if s is a compound \<state\> and one of its children is an active <final> state
    /// (i.e. is a member of the current configuration), or if s is a \<parallel\> state and
    /// isInFinalState is true of all its children.
    /// ```
    /// function isInFinalState(s):
    ///     if isCompoundState(s):
    ///         return getChildStates(s).some(lambda s: isFinalState(s) and configuration.isMember(s))
    ///     elif isParallelState(s):
    ///         return getChildStates(s).every(isInFinalState)
    ///     else:
    ///         return false
    /// ```
    /// #Actual implementation:
    ///  todo (check argument types!)
    fn isInFinalState(&self, state: StateId) -> bool { todo!() }


    /// #W3C says:
    /// # function getTransitionDomain(transition)
    /// Return the compound state such that
    /// 1) all states that are exited or entered as a result of taking 'transition'
    ///    are descendants of it
    /// 2) no descendant of it has this property.
    /// ```
    /// function getTransitionDomain(t)
    ///     tstates = getEffectiveTargetStates(t)
    ///     if not tstates:
    ///         return null
    ///     elif t.type == "internal" and isCompoundState(t.source) and tstates.every(lambda s: isDescendant(s,t.source)):
    ///         return t.source
    ///     else:
    ///         return findLCCA([t.source].append(tstates))
    /// ```
    /// #Actual implementation:
    /// No "Option" here, as "StateId" can be "0" to identify "none".
    fn getTransitionDomain(&self, t: &Transition) -> StateId {
        let tstates = self.getEffectiveTargetStates(t);
        if tstates.isEmpty() {
            0
        } else if t.transition_type == TransitionType::Internal &&
            self.isCompoundState(t.source) && tstates.every(&|s| -> bool { self.isDescendant(*s, t.source) })
        {
            t.source
        } else {
            let mut l = List::new();
            l.push(t.source);
            l.appendSet(&tstates);
            self.findLCCA(&l)
        }
    }

    /// #W3C says:
    /// # function findLCCA(stateList)
    /// The Least Common Compound Ancestor is the \<state\> or \<scxml\> element s such that s is a
    /// proper ancestor of all states on stateList and no descendant of s has this property.
    /// Note that there is guaranteed to be such an element since the <scxml> wrapper element is a
    /// common ancestor of all states. Note also that since we are speaking of proper ancestor
    /// (parent or parent of a parent, etc.) the LCCA is never a member of stateList.
    /// ```
    /// function findLCCA(stateList):
    ///     for anc in getProperAncestors(stateList.head(),null).filter(isCompoundStateOrScxmlElement):
    ///         if stateList.tail().every(lambda s: isDescendant(s,anc)):
    ///             return anc
    /// ```
    /// #Actual implementation:
    fn findLCCA(&self, stateList: &List<StateId>) -> StateId {
        for anc in self.getProperAncestors(*stateList.head(), 0)
            .toList().filterBy(&|s| { self.isCompoundStateOrScxmlElement(*s) }).iterator() {
            if stateList.tail().every(&|s| { self.isDescendant(*s, *anc) }) {
                return *anc;
            }
        }
        0
    }

    /// #W3C says:
    /// # function getEffectiveTargetStates(transition)
    /// Returns the states that will be the target when 'transition' is taken, dereferencing any history states.
    /// ```
    /// function getEffectiveTargetStates(transition)
    ///     targets = new OrderedSet()
    ///     for s in transition.target
    ///         if isHistoryState(s):
    ///             if historyValue[s.id]:
    ///                 targets.union(historyValue[s.id])
    ///             else:
    ///                 targets.union(getEffectiveTargetStates(s.transition))
    ///         else:
    ///             targets.add(s)
    ///     return targets
    /// ```
    /// #Actual implementation:
    ///  todo (check argument types!)
    fn getEffectiveTargetStates(&self, transition: &Transition) -> OrderedSet<StateId> {
        let mut targets: OrderedSet<StateId> = OrderedSet::new();
        for sid in &transition.target {
            if self.isHistoryState(*sid) {
                if self.historyValue.has(*sid) {
                    targets.union(self.historyValue.get(*sid));
                } else {
                    let s = self.get_state_by_id(*sid);
                    // History states have excatly one "transition"
                    targets.union(&self.getEffectiveTargetStates(self.get_transition_by_id(*s.transitions.head())));
                }
            } else {
                targets.add(*sid);
            }
        }
        targets
    }

    /// #W3C says:
    /// # function getProperAncestors(state1, state2)
    /// If state2 is null, returns the set of all ancestors of state1 in ancestry order (state1's parent followed by the parent's parent, etc. up to an including the <scxml> element). If state2 is non-null, returns in ancestry order the set of all ancestors of state1, up to but not including state2. (A "proper ancestor" of a state is its parent, or the parent's parent, or the parent's parent's parent, etc.))If state2 is state1's parent, or equal to state1, or a descendant of state1, this returns the empty set.
    fn getProperAncestors(&self, state1: StateId, state2: StateId) -> OrderedSet<StateId> {
        let mut properAncestors: OrderedSet<StateId> = OrderedSet::new();
        if !self.isDescendant(state2, state1) {
            let mut currState = self.get_state_by_id(state1).parent;
            while currState != 0 && currState != state2 {
                properAncestors.add(currState);
                currState = self.get_state_by_id(currState).parent;
            }
        }
        properAncestors
    }

    /// #W3C says:
    /// function isDescendant(state1, state2)
    /// Returns 'true' if state1 is a descendant of state2 (a child, or a child of a child, or a child of a child of a child, etc.) Otherwise returns 'false'.
    fn isDescendant(&self, state1: StateId, state2: StateId) -> bool {
        if state1 == 0 || state2 == 0 || state1 == state2 {
            false
        } else {
            let mut currState = self.get_state_by_id(state1).parent;
            while currState != 0 && currState != state2 {
                currState = self.get_state_by_id(currState).parent;
            }
            currState != state2
        }
    }

    fn isCompoundState(&self, state: StateId) -> bool {
        if state != 0 {
            !self.get_state_by_id(state).states.is_empty()
        } else {
            false
        }
    }

    fn isCompoundStateOrScxmlElement(&self, sid: StateId) -> bool {
        sid == self.pseudo_root || !self.get_state_by_id(sid).states.is_empty()
    }

    fn isHistoryState(&self, state: StateId) -> bool {
        self.get_state_by_id(state).history_type != HistoryType::None
    }

    fn isCancelEvent(&self, ev: &Event) -> bool {
        // Cancel-Events (outer fsm cancels a fsm instance that was started by some invoke)
        // are platform specific.
        // TODO: we need a "invoke" concept!
        ev.name.ends_with(".cancel")
    }


    /// #W3C says:
    /// function getChildStates(state1)
    /// Returns a list containing all \<state\>, \<final\>, and \<parallel\> children of state1.
    /// #Actual implementation:
    ///  todo (check argument types!)
    fn getChildStates(&self, state1: StateId) -> List<StateId> {
        let mut l: List<StateId> = List::new();
        let stateRef = self.get_state_by_id(state1);
        for c in &stateRef.states {
            l.push(*c);
        }
        l
    }

    fn invoke(&mut self, inv: &Invoke) {
        // TODO: we need a "invoke" concept!
    }

    fn cancelInvoke(&mut self, inv: InvokeId) {
        // TODO: we need a "invoke" concept!
        // Send a cancel event to the thread/pricess.
        // see isCancelEvent
    }

    fn applyFinalize(&mut self, invokeId: InvokeId, event: &Event) {
        todo!()
    }

    fn send(&mut self, invokeId: InvokeId, event: &Event) {
        todo!()
    }

    fn conditionMatch(&self, t: &Transition) -> bool
    {
        if t.content != 0 {
            todo!()
        }
        true
    }

    fn nameMatch(&self, events: &Vec<String>, name: &String) -> bool
    {
        println!("nameMatch {} in {:?}", name, events);
        events.contains(name)
    }
}

pub type StateId = u32;

#[derive(Debug)]
struct EmptyData {}

impl EmptyData {
    pub fn new() -> EmptyData {
        EmptyData {}
    }
}

impl Data for EmptyData {
    fn get_copy(&self) -> Box<dyn Data> {
        Box::new(EmptyData {})
    }
}

#[derive(Debug)]
pub struct DataStore {
    pub values: HashMap<String, Box<dyn Data>>,

    nullValue: Box<dyn Data>,
}

impl DataStore {
    fn new() -> DataStore {
        DataStore {
            values: HashMap::new(),
            nullValue: Box::new(EmptyData::new()),
        }
    }

    fn get(&self, key: &String) -> &Box<dyn Data> {
        if self.values.contains_key(key) {
            self.values.get(key).unwrap()
        } else {
            &self.nullValue
        }
    }

    fn set(&mut self, key: &String, data: Box<dyn Data>) {
        self.values.insert(key.clone(), data);
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct DoneData {}

/// Stores all data for a State.
/// In this model "State" is used for SCXML elements "State" and "Parallel".
///
/// ## W3C says:
/// 3.3 \<state\>
/// Holds the representation of a state.
///
/// 3.3.1 Attribute Details
///
/// |Name| Required| Attribute Constraints|Type|Default Value|Valid Values|Description|
/// |----|---------|----------------------|----|-------------|------------|-----------|
/// |id|false|none|ID|none|A valid id as defined in [https://www.w3.org/TR/scxml/#Schema](XML Schema)|The identifier for this state. See 3.14 IDs for details.|
/// |initial|false|MUST NOT be specified in conjunction with the \<initial\> element. MUST NOT occur in atomic states.|IDREFS|none|A legal state specification. See 3.11 Legal State Configurations and Specifications for details.|The id of the default initial state (or states) for this state.|
///
/// 3.3.2 Children
/// - \<onentry\> Optional element holding executable content to be run upon entering this <state>.
///   Occurs 0 or more times. See 3.8 \<onentry\>
/// - \<onexit\> Optional element holding executable content to be run when exiting this <state>.
///   Occurs 0 or more times. See 3.9 \<onexit\>
/// - \<transition\> Defines an outgoing transition from this state. Occurs 0 or more times.
///   See 3.5 <transition>
/// - \<initial\> In states that have substates, an optional child which identifies the default
///   initial state. Any transition which takes the parent state as its target will result in the
///   state machine also taking the transition contained inside the <initial> element.
///   See 3.6 \<initial\>
/// - \<state\> Defines a sequential substate of the parent state. Occurs 0 or more times.
/// - \<parallel\> Defines a parallel substate. Occurs 0 or more times. See 3.4 \<parallel\>
/// - \<final\>. Defines a final substate. Occurs 0 or more times. See 3.7 \<final\>.
/// - \<history\> A child pseudo-state which records the descendant state(s) that the parent state
///   was in the last time the system transitioned from the parent.
///   May occur 0 or more times. See 3.10 \<history\>.
/// - \<datamodel\> Defines part or all of the data model. Occurs 0 or 1 times. See 5.2 \<datamodel\>
/// - \<invoke> Invokes an external service. Occurs 0 or more times. See 6.4 \<invoke\> for details.
///
/// ##Definitions:
/// - An atomic state is a \<state\> that has no \<state\>, \<parallel\> or \<final\> children.
/// - A compound state is a \<state\> that has \<state\>, \<parallel\>, or \<final\> children
///   (or a combination of these).
/// - The default initial state(s) of a compound state are those specified by the 'initial' attribute
///   or \<initial\> element, if either is present. Otherwise it is the state's first child state
///   in document order.
///
/// In a conformant SCXML document, a compound state may specify either an "initial" attribute or an
/// \<initial\> element, but not both. See 3.6 \<initial\> for a discussion of the difference between
/// the two notations.
pub struct State {
    /// The internal Id (not W3C). Used to refence the state.
    pub id: StateId,

    /// The SCXML id.
    pub name: String,

    /// The initial transition id (if the state has sub-states).
    pub initial: TransitionId,

    /// The Ids of the sub-states of this state.
    pub states: Vec<StateId>,

    /// True for "parallel" states
    pub is_parallel: bool,

    /// True for "final" states
    pub is_final: bool,

    pub history_type: HistoryType,

    /// The script that is executed if the state is entered. See W3c comments for \<onentry\> above.
    pub onentry: List<ExecutableContentId>,

    /// The script that is executed if the state is left. See W3c comments for \<onexit\> above.
    pub onexit: List<ExecutableContentId>,

    /// All transitions between sub-states.
    pub transitions: List<TransitionId>,

    pub invoke: List<Invoke>,
    pub history: List<StateId>,

    /// The local datamodel
    pub datamodel: Box<dyn Datamodel>,
    pub data: DataStore,

    pub isFirstEntry: bool,
    pub parent: StateId,
    pub donedata: Option<DoneData>,
}

impl State {
    pub fn new(name: &String) -> State {
        let idc = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        State {
            id: idc,
            name: name.clone(),
            initial: 0,
            states: vec![],
            onentry: List::new(),
            onexit: List::new(),
            transitions: List::new(),
            is_parallel: false,
            is_final: false,
            history_type: HistoryType::None,
            /// True if the state was never entered before.
            datamodel: Box::new(NullDatamodel::new()),
            data: DataStore::new(),
            isFirstEntry: false,
            parent: 0,
            donedata: None,
            invoke: List::new(),
            history: List::new(),
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
pub enum HistoryType {
    Shallow,
    Deep,
    None,
}

pub fn map_history_type(ts: &String) -> HistoryType {
    match ts.to_lowercase().as_str() {
        "deep" => HistoryType::Deep,
        "shallow" => HistoryType::Shallow,
        "" => HistoryType::None,
        _ => panic!("Unknown transition type '{}'", ts),
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum TransitionType {
    Internal,
    External,
}

pub fn map_transition_type(ts: &String) -> TransitionType {
    match ts.to_lowercase().as_str() {
        "internal" => TransitionType::Internal,
        "external" => TransitionType::External,
        "" => TransitionType::External,
        _ => panic!("Unknown transition type '{}'", ts),
    }
}

static ID_COUNTER: AtomicU32 = AtomicU32::new(1);

pub type TransitionId = u32;

#[derive(Debug)]
pub struct Transition {
    pub id: TransitionId,

    // TODO: Possibly we need some type to express event ids
    pub events: Vec<String>,
    pub cond: Option<Box<dyn ConditionalExpression>>,
    pub source: StateId,
    pub target: Vec<StateId>,
    pub transition_type: TransitionType,
    pub content: ExecutableContentId,
}

impl PartialEq for Transition {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }

    fn ne(&self, other: &Self) -> bool {
        self.id != other.id
    }
}

impl Transition {
    pub fn new() -> Transition {
        let idc = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Transition {
            id: idc,
            events: vec![],
            cond: None,
            source: 0,
            target: vec![],
            transition_type: TransitionType::Internal,
            content: 0,
        }
    }
}

pub trait Data: Send + Debug {
    fn get_copy(&self) -> Box<dyn Data>;
}

pub trait Datamodel: Send + Debug {
    fn get_name(self: &Self) -> &str;
    fn initializeDataModel(self: &mut Self, data: &DataStore);
    fn set(&mut self, name: &String, data: Box<dyn Data>);
    fn get(&self, name: &String) -> &dyn Data;
    fn clear(&mut self);
    fn log(&mut self, msg: &String);
}

pub fn createDatamodel(name: &str) -> Box<dyn Datamodel> {
    match name.to_lowercase().as_str() {
        ECMA_SCRIPT_LC => Box::new(ECMAScriptDatamodel::new()),
        NULL_DATAMODEL_LC => Box::new(NullDatamodel::new()),
        _ => panic!("Unsupported Datamodel '{}'", name)
    }
}

pub type InvokeId = u32;

#[derive(Debug)]
#[derive(Clone, PartialEq)]
pub struct Invoke {
    pub id: InvokeId,
    pub name: String,
    pub invokeid: InvokeId,
    pub autoforward: bool,
    // TODO
}

/**
 * ECMAScript data model
 */
#[derive(Debug)]
pub struct ECMAScriptDatamodel {
    pub data: DataStore,

}

impl ECMAScriptDatamodel {
    pub fn new() -> ECMAScriptDatamodel {
        ECMAScriptDatamodel { data: DataStore::new() }
    }
}

impl Datamodel for ECMAScriptDatamodel {
    fn get_name(self: &Self) -> &str {
        return ECMA_SCRIPT;
    }

    fn initializeDataModel(&mut self, data: &DataStore) {
        for (name, data) in &data.values
        {
            self.data.values.insert(name.clone(), data.deref().get_copy());
        }
    }

    fn set(self: &mut ECMAScriptDatamodel, name: &String, data: Box<dyn Data>) {
        self.data.set(name, data);
    }

    fn get(self: &ECMAScriptDatamodel, name: &String) -> &dyn Data {
        todo!()
    }

    fn clear(self: &mut ECMAScriptDatamodel) {}

    fn log(&mut self, msg: &String) {
        println!("Log: {}", msg);
    }
}

/// ## W3C says:
/// ###B.1 The Null Data Model
/// The value "null" for the 'datamodel' attribute results in an absent or empty data model. In particular:
/// - B.1.1 Data Model
///
///   There is no underlying data model.
/// - B.1.2 Conditional Expressions
///
///   The boolean expression language consists of the In predicate only. It has the form 'In(id)',
///   where id is the id of a state in the enclosing state machine.
///   The predicate must return 'true' if and only if that state is in the current state configuration.
/// - B.1.3 Location Expressions
///
///   There is no location expression language.
/// - B.1.4 Value Expressions
///
///   There is no value expression language.
/// - B.1.5 Scripting
///
///   There is no scripting language.
/// - B.1.6 System Variables
///
///   System variables are not accessible.
/// - B.1.7 Unsupported Elements
///
///   The \<foreach\> element and the elements defined in 5 Data Model and Data Manipulation are not
///   supported in the Null Data Model.
#[derive(Debug)]
pub struct NullDatamodel {}

impl NullDatamodel {
    pub fn new() -> NullDatamodel {
        NullDatamodel {}
    }
}

impl Datamodel for NullDatamodel {
    fn get_name(self: &Self) -> &str {
        return NULL_DATAMODEL;
    }
    fn initializeDataModel(self: &mut Self, data: &DataStore) {}

    fn set(self: &mut NullDatamodel, name: &String, data: Box<dyn Data>) {
        todo!()
    }

    fn get(self: &NullDatamodel, name: &String) -> &dyn Data {
        todo!()
    }

    fn clear(self: &mut NullDatamodel) {}

    fn log(self: &mut NullDatamodel, msg: &String) {
        println!("Log: {}", msg);
    }
}


/// A boolean expression, interpreted by the used datamodel-language.
pub trait ConditionalExpression: Send + Debug {
    fn execute(self: &Self, data: &dyn Datamodel) -> bool { false }
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
    fn execute(self: &Self, data: &dyn Datamodel) -> bool {
        return true;
    }
}

pub type ExecutableContentId = u32;

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
        write!(f, "Fsm{{v:{} root:{} states:", self.version, self.pseudo_root)?;
        display_state_map(&self.states, f)?;
        display_transition_map(&self.transitions, f)?;
        write!(f, "}}")
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{#{} states:{} transitions: {}}}", self.id, idVecToString(&self.states),
               idVecToString(&self.transitions.data)
        )
    }
}

impl Display for Transition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{#{} type:{} events:{:?} cond:{:?} target:{:?} }}",
               self.id,
               self.transition_type, &self.events, self.cond,
               self.target)
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

fn idVecToString(v: &Vec<u32>) -> String {
    let mut s = "[".to_string();

    let len = v.len();
    for i in 0..len {
        s += format!("{}{}",
                     if i > 0 {
                         ","
                     } else {
                         ""
                     }, v[i]).as_str();
    }
    s += "]";
    s
}