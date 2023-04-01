use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::atomic::{AtomicU32, Ordering};

use log::debug;
use quick_xml::events::{BytesStart, Event};
use quick_xml::events::attributes::Attributes;
use quick_xml::Reader;

use crate::executable_content::{ExecutableContent, Expression, Log, SendParameters};
use crate::fsm::{ExecutableContentId, Fsm, HistoryType, map_history_type, map_transition_type, Name, State, StateId, Transition, TransitionId, TransitionType};
use crate::fsm::vecToString;

pub type AttributeMap = HashMap<String, String>;

static DOC_ID_COUNTER: AtomicU32 = AtomicU32::new(1);


/// #W3C says:
/// The top-level wrapper element, which carries version information. The actual state machine consists of its children.
/// Note that only one of the children is active at any one time. See 3.11 Legal State Configurations and Specifications for details.\
/// *Attributes:*
/// + __initial__ A legal state specification. See 3.11 Legal State Configurations and Specifications for details. If not specified, the default initial state is the first child state in document order.
/// + __name__ Any valid NMTOKEN. The name of this state machine. It is for purely informational purposes.
/// + __xmlns__   The value MUST be "http://www.w3.org/2005/07/scxml".
/// + __version__   Decimal, The value MUST be "1.0".
/// + __datamodel__ NMTOKEN, platform-specific, "null", "ecmascript", "xpath" or other platform-defined values.
/// + __binding__  "early" or "late", default is "early". See 5.3.3 Data Binding for details.
///
/// *Children:*
/// + __state__ A compound or atomic state. Occurs zero or more times. See 3.3 \<state\> for details.
/// + __parallel__  A parallel state. Occurs zero or more times. See 3.4 \<parallel\> for details.
/// + __final__  A top-level final state in the state machine. Occurs zero or more times. The SCXML processor must terminate processing when the state machine reaches this state. See 3.7 \<final\> for details.
/// + __datamodel__  Defines part or all of the data model. Occurs 0 or 1 times. See 5.2 \<datamodel\>
/// + __script__ Provides scripting capability. Occurs 0 or 1 times. 5.8 \<script\>
pub const TAG_SCXML: &str = "scxml";

pub const ATTR_DATAMODEL: &str = "datamodel";

pub const TAG_DATAMODEL: &str = "datamodel";
pub const TAG_DATA: &str = "data";
pub const TAG_VERSION: &str = "version";
pub const TAG_INITIAL: &str = "initial";
pub const ATTR_ID: &str = "id";

/// #W3C says:
/// Holds the representation of a state.
///
/// *Attributes*:
/// + __id__  valid id as defined in [XML Schema].The identifier for this state. See 3.14 IDs for details.
/// + __initial__	The id of the default initial state (or states) for this state. MUST NOT be specified in conjunction with the \<initial\> element. MUST NOT occur in atomic states.
///
/// *Children*:
/// + __onentry__ Optional element holding executable content to be run upon entering this state. Occurs 0 or more times.
/// + __onexit__ Optional element holding executable content to be run when exiting this state. Occurs 0 or more times.
/// + __transition__ Defines an outgoing transition from this state. Occurs 0 or more times.
/// + __initial__ In states that have substates, an optional child which identifies the default initial state.
///   Any transition which takes the parent state as its target will result in the state machine also taking the transition
///   contained inside the \<initial\> element.
/// + __state__ Defines a sequential substate of the parent state. Occurs 0 or more times.
/// + __parallel__ Defines a parallel substate. Occurs 0 or more times.
/// + __final__  Defines a final substate. Occurs 0 or more times.
/// + __history__ A child pseudo-state which records the descendant state(s) that the parent state was in the last time the system transitioned from the parent.
///   May occur 0 or more times.
/// + __datamodel__ Defines part or all of the data model. Occurs 0 or 1 times.
/// + __invoke__ Invokes an external service. Occurs 0 or more times.
///
/// [__Definition__: An atomic state is a <state> that has no <state>, <parallel> or <final> children.]\
/// [__Definition__: A compound state is a <state> that has <state>, <parallel>, or <final> children (or a combination of these).]\
/// [__Definition__: The default initial state(s) of a compound state are those specified by the 'initial' attribute or <initial> element, if either is present. Otherwise it is the state's first child state in document order. ]\
/// In a conformant SCXML document, a compound state may specify either an "initial" attribute or an <initial> element, but not both.
/// See 3.6 \<initial\> for a discussion of the difference between the two notations.
///
pub const TAG_STATE: &str = "state";
pub const ATTR_INITIAL: &str = "initial";
pub const TAG_HISTORY: &str = "history";
pub const TAG_PARALLEL: &str = "parallel";
pub const TAG_FINAL: &str = "final";
pub const TAG_TRANSITION: &str = "transition";
pub const TAG_COND: &str = "cond";
pub const TAG_EVENT: &str = "event";
pub const TAG_TARGET: &str = "target";
pub const TAG_TYPE: &str = "type";
pub const TAG_ON_ENTRY: &str = "onentry";
pub const TAG_ON_EXIT: &str = "onexit";
pub const TAG_INVOKE: &str = "invoke";
pub const TA_DONEDATA: &str = "donedata";

pub const TAG_INCLUDE: &str = "include";
pub const TAG_HREF: &str = "href";
pub const ATTR_PARSE: &str = "parse";
pub const ATTR_XPOINTER: &str = "xpointer";

/// Executable content
pub const TAG_RAISE: &str = "raise";

/// #W3C says:
/// __\<send\>__ is used to send events and data to external systems, including external SCXML Interpreters, or to raise events in the current SCXML session.
///
/// *Attributes*:
/// + __event__	A string indicating the name of message being generated. Must not occur with 'eventexpr'. If the type is *http:\/\/www\.w3.org/TR/scxml/#SCXMLEventProcessor*, either
///             this attribute or 'eventexpr' must be present.
/// + __eventexpr__	A dynamic alternative to 'event'. If this attribute is present, the SCXML Processor must evaluate it when the parent \<send\> element is evaluated and treat
///                 the result as if it had been entered as the value of 'event'. 
///                 If the type is "*http:\/\/www\.w3.org/TR/scxml/#SCXMLEventProcessor*", either this attribute or 'event' must be present. Must not occur with 'event'.
/// + __target__	A valid target URI. The unique identifier of the message target that the platform should send the event to. Must not occur with 'targetexpr'.
///                 See [6.2.4 The Target of Send](https://www.w3.org/TR/scxml/#SendTargets) for details.
///                 See [SCXMLEventProcessor](https://www.w3.org/TR/scxml/#SCXMLEventProcessor) for details about predefined targets.
/// + __targetexpr__ An expression evaluating to a valid target URI	A dynamic alternative to 'target'. If this attribute is present, the SCXML Processor must evaluate it when the parent \<send\> element is evaluated and treat the result as if it
///                  had been entered as the value of 'target'. Must not occur with 'target'.
/// + __type__	    The URI that identifies the transport mechanism for the message. Must not occur with 'typeexpr'.
///                 See [6.2.5 The Type of Send](https://www.w3.org/TR/scxml/#SendTypes).
/// + __typeexpr__	A dynamic alternative to 'type'. If this attribute is present, the SCXML Processor must evaluate it when the parent \<send\> element is evaluated and treat the result as if it had been
///                 entered as the value of 'type'. Must not occur with 'type'.
/// + __id__	Any valid token	A string literal to be used as the identifier for this instance of <send>. Must not occur with 'idlocation'.
/// + __idlocation__ Any location expression evaluating to a data model location in which a system-generated id can be stored. See below for details. Must not occur with 'id'.
/// + __delay__	A time designation as defined in CSS2 format (RegExp: "\\d*(\\.\\d+)?(ms|s|m|h|d))").
///             Indicates how long the processor should wait before dispatching the message.
///             Must not occur with 'delayexpr' or when the attribute 'target' has the value "_internal".
/// + __delayexpr__	A value expression which returns a time designation as defined in CSS2 format. A dynamic alternative to 'delay'. If this attribute is present, the SCXML
///                 Processor must evaluate it when the parent \<send\> element is evaluated and treat the result as if it had been entered as the value of 'delay'.
///                 Must not occur with 'delay' or when the attribute 'target' has the value "_internal".
/// + __namelist__	A space-separated list of one or more data model locations to be included as attribute/value pairs with the message. (The name of the location is the attribute
///                 and the value stored at the location is the value.).
///                 Must not be specified in conjunction with \<content\> element.
///
/// *Children*
/// + __param__ The SCXML Processor must evaluate this element when the parent \<send\> element is evaluated and pass the resulting data to the external service when the message
///             is delivered. Occurs 0 or more times.
/// + __content__ The SCXML Processor must evaluate this element when the parent \<send\> element is evaluated and pass the resulting data to the external service when the message
///             is delivered. Occurs 0 or 1 times.
///
/// A conformant SCXML document must specify exactly one of 'event', 'eventexpr' and \<content\>. A conformant document must not specify "namelist" or \<param\> with \<content\>.\
/// The SCXML Processor must include all attributes and values provided by \<param\> or 'namelist' even if duplicates occur.\
/// If 'idlocation' is present, the SCXML Processor must generate an id when the parent \<send\> element is evaluated and store it in this location. See [3.14 IDs](https://www.w3.org/TR/scxml/#IDs) for details.\
/// If a delay is specified via 'delay' or 'delayexpr', the SCXML Processor must interpret the character string as a time interval. It must dispatch the message only when the delay interval elapses.
/// (Note that the evaluation of the send tag will return immediately.)\
/// The Processor must evaluate all arguments to <send> when the <send> element is evaluated, and not when the message is actually dispatched. If the evaluation of \<send\>'s arguments produces an error,
/// the Processor must discard the message without attempting to deliver it. If the SCXML session terminates before the delay interval has elapsed, the SCXML Processor must discard the message without
/// attempting to deliver it.
pub const TAG_SEND: &str = "send";

pub const ATTR_EVENT: &str = "event";
pub const ATTR_EVENTEXPR: &str = "eventexpr";
pub const ATTR_TARGET: &str = "target";
pub const ATTR_TARGETEXPR: &str = "targetexpr";
pub const ATTR_TYPE: &str = "type";
pub const ATTR_TYPEEXPR: &str = "typeexpr";
pub const ATTR_IDLOCATION: &str = "idlocation";
pub const ATTR_DELAY: &str = "delay";
pub const ATTR_DELAYEXPR: &str = "delayexpr";
pub const ATTR_NAMELIST: &str = "namelist";
pub const TAG_PARAM: &str = "param";
pub const TAG_CONTENT: &str = "content";

pub const TAG_LOG: &str = "log";
pub const TAG_SCRIPT: &str = "script";
pub const ATTR_SRC: &str = "src";
pub const TAG_ASSIGN: &str = "assign";
pub const TAG_IF: &str = "if";
pub const TAG_FOR_EACH: &str = "foreach";
pub const ATTR_ARRAY: &str = "array";
pub const ATTR_ITEM: &str = "item";
pub const ATTR_INDEX: &str = "index";

pub const TAG_CANCEL: &str = "cancel";
pub const TAG_ELSE: &str = "else";
pub const TAG_ELSEIF: &str = "elseif";
pub const ATTR_EXPR: &str = "expr";

pub const NS_XINCLUDE: &str = "http://www.w3.org/2001/XInclude";

struct ReaderStackItem {
    current_state: StateId,
    current_transition: TransitionId,
    current_tag: String,
    current_executable_content: ExecutableContentId,
}

impl ReaderStackItem {
    pub fn new(o: &ReaderStackItem) -> ReaderStackItem {
        ReaderStackItem {
            current_state: o.current_state,
            current_transition: o.current_transition,
            current_tag: o.current_tag.clone(),
            current_executable_content: o.current_executable_content,
        }
    }
}


struct ReaderState {
    // True if reader in inside an scxml element
    in_scxml: bool,
    id_count: i32,
    file: String,

    // The resulting fsm
    fsm: Box<Fsm>,

    current: ReaderStackItem,
    stack: Vec<ReaderStackItem>,
}


impl ReaderState {
    pub fn new(f: &String) -> ReaderState {
        ReaderState {
            in_scxml: false,
            id_count: 0,
            stack: vec![],
            current: ReaderStackItem {
                current_state: 0,
                current_transition: 0,
                current_tag: "".to_string(),
                current_executable_content: 0,

            },
            fsm: Box::new(Fsm::new()),
            file: f.clone(),
        }
    }

    fn push(&mut self, tag: &str) {
        self.stack.push(ReaderStackItem::new(&self.current));
        self.current.current_tag = tag.to_string();
    }

    fn pop(&mut self) {
        let p = self.stack.pop();
        if p.is_some() {
            self.current = p.unwrap();
        }
    }

    fn generate_name(&mut self) -> String {
        self.id_count += 1;
        format!("__id{}", self.id_count)
    }

    fn parse_state_specification(&mut self, target_name: &str, targets: &mut Vec<StateId>) {
        target_name.split_ascii_whitespace().for_each(|target| {
            targets.push(self.get_or_create_state(&target.to_string(), false))
        });
    }

    fn get_state_by_name(&self, name: &Name) -> Option<&State> {
        if self.fsm.global().borrow().statesNames.contains_key(name) {
            Some(self.fsm.get_state_by_name(name))
        } else { None }
    }

    fn get_state_by_name_mut(&mut self, name: &Name) -> Option<&mut State> {
        if self.fsm.global().borrow().statesNames.contains_key(name) {
            Some(self.fsm.get_state_by_name_mut(name))
        } else { None }
    }

    fn get_state_by_id(&self, id: StateId) -> &State {
        self.fsm.get_state_by_id(id)
    }

    fn get_state_by_id_mut(&mut self, id: StateId) -> &mut State {
        self.fsm.get_state_by_id_mut(id)
    }

    fn get_current_state(&mut self) -> &mut State {
        let id = self.current.current_state;
        if id <= 0 {
            panic!("Internal error: Current State is unknown");
        }
        self.get_state_by_id_mut(id)
    }


    fn get_current_transition(&mut self) -> &mut Transition {
        let id = self.current.current_transition;
        if id <= 0 {
            panic!("Internal error: Current Transition is unknown");
        }
        self.fsm.get_transition_by_id_mut(id)
    }

    fn add_executable_content(&mut self, ec: Box<dyn ExecutableContent>) -> ExecutableContentId {
        if self.current.current_executable_content == 0 {
            self.current.current_executable_content = ec.deref().get_id();
        } else {
            let global = self.fsm.datamodel.global();
            let mut gb = global.borrow_mut();
            let ex = gb.executableContent.get_mut(&self.current.current_executable_content).unwrap();

            todo!()
        }
        self.fsm.global().borrow_mut().executableContent.insert(ec.deref().get_id(), ec);
        self.current.current_executable_content
    }

    fn get_parent_tag(&self) -> &str {
        let mut r = "";
        if !self.stack.is_empty() {
            r = self.stack.get(self.stack.len() - 1).as_ref().unwrap().current_tag.as_str();
        }
        r
    }

    pub fn verify_parent_tag(&self, name: &str, allowed_parents: &[&str]) -> &str {
        let parent_tag = self.get_parent_tag();
        if !allowed_parents.contains(&parent_tag) {
            let mut allowed_parents_s = "".to_string();
            let len = allowed_parents.len();
            for i in 0..allowed_parents.len() {
                allowed_parents_s += format!("{}<{}>",
                                             if i > 0 {
                                                 if i < (len - 1) {
                                                     ", "
                                                 } else {
                                                     " or "
                                                 }
                                             } else {
                                                 ""
                                             }, allowed_parents[i]).as_str();
            }
            panic!("<{}> inside <{}>. Only allowed inside {}", name, parent_tag,
                   allowed_parents_s);
        }
        parent_tag
    }


    fn get_or_create_state(&mut self, name: &String, parallel: bool) -> StateId {
        let m = self.fsm.global().borrow().statesNames.get(name).cloned();
        match m {
            None => {
                let mut s = State::new(name);
                s.id = (self.fsm.states.len() + 1) as StateId;
                s.is_parallel = parallel;
                let sid = s.id;
                let gd = self.fsm.global();
                gd.borrow_mut().statesNames.insert(s.name.clone(), s.id); // s.id, s);
                self.fsm.states.push(s);
                sid
            }
            Some(id) => {
                if parallel {
                    self.fsm.states.get_mut((id - 1) as usize).unwrap().is_parallel = true;
                }
                id
            }
        }
    }

    fn get_or_create_state_with_attributes(&mut self, attr: &AttributeMap, parallel: bool, parent: StateId) -> StateId {
        let sname: String;
        match attr.get(ATTR_ID) {
            None => sname = self.generate_name(),
            Some(id) => sname = id.clone()
        }
        let id = self.get_or_create_state(&sname, parallel);

        let initial;
        match attr.get(ATTR_INITIAL) {
            None => initial = 0,
            Some(id_refs) => {
                // Create initial-transition with the initial states
                let mut t = Transition::new();
                t.doc_id = DOC_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
                t.transition_type = TransitionType::Internal;
                t.source = id;
                initial = t.id;
                self.parse_state_specification(id_refs, &mut t.target);
                debug!(" {}#{}.initial = {} -> {}", sname, id, initial, vecToString(&t.target));
                self.fsm.transitions.insert(t.id, t);
            }
        }

        let state = self.get_state_by_id_mut(id);
        if initial != 0 {
            state.initial = initial;
        }
        state.doc_id = DOC_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        if parent != 0 {
            state.parent = parent;
            let parent_state = self.get_state_by_id_mut(parent);
            if !parent_state.states.contains(&id) {
                parent_state.states.push(id);
            }
        }
        id
    }

    fn get_required_attr<'a>(tag: &str, attribute: &str, attr: &'a AttributeMap) -> &'a String {
        let attr = attr.get(attribute);
        if attr.is_none() {
            panic!("<{}> requires attribute {}", tag, attribute);
        }
        attr.unwrap()
    }

    fn read_from_uri(&mut self, uri: &String) -> Result<String, String> {
        let url_result = reqwest::Url::parse(uri);
        match url_result {
            Ok(url) => {
                println!("URL {}", url);

                let resp = reqwest::blocking::get(url);
                match resp {
                    Ok(r) => {
                        match r.text() {
                            Ok(s) => {
                                Ok(s)
                            }
                            Err(e) => {
                                Err(format!("Failed to decode from {}. {}", uri, e))
                            }
                        }
                    }
                    Err(e) => {
                        Err(format!("Failed to download {}. {}", uri, e))
                    }
                }
            }
            Err(e) => {
                debug!("{} is not a URI ({}). Try loading as relative path...", uri, e);
                let file_src = self.get_resolved_path(uri);
                match File::open(file_src.clone()) {
                    Ok(mut file) => {
                        let mut buf = String::with_capacity(file.metadata().unwrap().len() as usize);
                        match file.read_to_string(&mut buf) {
                            Ok(r) => {
                                Ok(buf)
                            }
                            Err(e) => {
                                Err(e.to_string())
                            }
                        }
                    }
                    Err(e) => {
                        Err(e.to_string())
                    }
                }
            }
        }
    }

    // A new "parallel" element started
    fn start_parallel(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_PARALLEL, TAG_SCXML);
        }
        let state_id = self.get_or_create_state_with_attributes(attr, true, self.current.current_state);
        if self.current.current_state > 0 {
            let parent_state = self.get_current_state();
            parent_state.states.push(state_id);
        }
        state_id
    }

    // A new "final" element started
    fn start_final(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_FINAL, TAG_SCXML);
        }
        let state_id = self.get_or_create_state_with_attributes(attr, false, self.current.current_state);

        self.fsm.get_state_by_id_mut(state_id).is_final = true;
        state_id
    }

    // A new "history" element started
    fn start_history(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_HISTORY, TAG_SCXML);
        }
        let state_id = self.get_or_create_state_with_attributes(attr, false, self.current.current_state);
        if self.current.current_state > 0 {
            let parent_state = self.get_current_state();
            parent_state.history.push(state_id);
        }
        let mut hstate = self.fsm.get_state_by_id_mut(state_id);

        match attr.get(TAG_TYPE) {
            None => hstate.history_type = HistoryType::Shallow,
            Some(type_name) => hstate.history_type = map_history_type(type_name)
        }
        state_id
    }

    // A new "state" element started
    fn start_state(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_STATE, TAG_SCXML);
        }
        let sid = self.get_or_create_state_with_attributes(&attr, false, self.current.current_state);
        self.current.current_state = sid;
        sid
    }

    // A "datamodel" element started (node, not attribute)
    fn start_datamodel(&mut self) {
        self.verify_parent_tag(TAG_DATAMODEL, &[TAG_SCXML, TAG_STATE, TAG_PARALLEL]);
        todo!()
    }

    fn start_data(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_DATA, &[TAG_DATAMODEL]);

        let id = Self::get_required_attr(TAG_DATA, ATTR_ID, attr);
        let src = attr.get(ATTR_SRC);
        let expr = attr.get(ATTR_EXPR);

        todo!()
    }

    /// A "initial" element started (node, not attribute)
    fn start_initial(&mut self) {
        self.verify_parent_tag(TAG_INITIAL, &[TAG_STATE, TAG_PARALLEL]);
        if self.get_current_state().initial > 0 {
            panic!("<{}> must not be specified if {}-attribute was given", TAG_INITIAL, ATTR_INITIAL)
        }
    }

    fn start_transition(&mut self, attr: &AttributeMap) {
        let parent_tag = self.verify_parent_tag(TAG_TRANSITION,
                                                &[TAG_HISTORY, TAG_INITIAL, TAG_STATE, TAG_PARALLEL]).to_string();

        let mut t = Transition::new();
        t.doc_id = DOC_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Start script.
        self.current.current_executable_content = 0;

        let event = attr.get(TAG_EVENT);
        if event.is_some() {
            t.events = event.unwrap().split_whitespace().map(|s| { s.to_string() }).collect();
        }

        let cond = attr.get(TAG_COND);
        if cond.is_some() {
            t.cond = Some(cond.unwrap().clone());
        }

        let target = attr.get(TAG_TARGET);
        match target {
            None => (),
            // TODO: Parse the state specification! (it can be a list)
            Some(target_name) => {
                self.parse_state_specification(target_name, &mut t.target);
            }
        }

        let trans_type = attr.get(TAG_TYPE);
        if trans_type.is_some() {
            t.transition_type = map_transition_type(trans_type.unwrap())
        }

        let state = self.get_current_state();

        if parent_tag.eq(TAG_INITIAL) {
            if state.initial > 0 {
                panic!("<initial> must not be specified if initial-attribute was given")
            }
            debug!(" {}#{}.initial = {}", state.name, state.id, t.id);
            state.initial = t.id;
        } else {
            state.transitions.push(t.id);
        }
        t.source = state.id;
        self.current.current_transition = t.id;
        self.fsm.transitions.insert(t.id, t);
    }

    fn end_transition(&mut self) {
        let ct_id = self.current.current_executable_content;
        self.current.current_executable_content = 0;

        let trans = self.get_current_transition();
        // Assign the collected content to the transition.
        trans.content = ct_id;
        self.current.current_transition = 0;
    }

    fn start_script(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_SCRIPT, &[TAG_SCXML, TAG_TRANSITION, TAG_ON_EXIT, TAG_ON_ENTRY, TAG_IF, TAG_FOR_EACH]);
        let src = attr.get(ATTR_SRC);
        if src.is_some() {
            let file_src = src.unwrap();
            // W3C:
            // If the script can not be downloaded within a platform-specific timeout interval,
            // the document is considered non-conformant, and the platform must reject it.
            match self.read_from_uri(file_src) {
                Ok(source) => {
                    let s = Box::new(Expression::new(source));
                    let ec_id = self.add_executable_content(s);

                    let state = self.get_current_state();
                    state.script_src = file_src.clone();
                    state.script = ec_id;
                    debug!("src='{}':\n{}", file_src, self.get_current_state().script );
                }
                Err(e) => {
                    panic!("Can't read script '{}'. {}", file_src, e);
                }
            }
        }
    }

    fn end_script(&mut self, txt: &mut Vec<String>) {
        let script_text = txt.concat();
        let src = script_text.trim();

        if !src.is_empty() {
            let ct_id = self.add_executable_content(Box::new(Expression::new(script_text)));
            let state = self.get_current_state();
            state.script = ct_id;

            if !state.script_src.is_empty() {
                panic!("<script> with 'src' attribute shall not have content.")
            }
        }
        txt.clear();
    }

    fn start_for_each(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_FOR_EACH, &[TAG_ON_ENTRY, TAG_ON_EXIT, TAG_TRANSITION, TAG_FOR_EACH, TAG_IF]);

        let array = Self::get_required_attr(TAG_FOR_EACH, ATTR_ARRAY, attr);
        let item = Self::get_required_attr(TAG_FOR_EACH, ATTR_ITEM, attr);
        let index = attr.get(ATTR_INDEX);

        todo!()
    }

    fn start_cancel(&mut self, attr: &AttributeMap) {
        todo!()
    }

    fn start_on_entry(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_ON_ENTRY, &[TAG_STATE, TAG_PARALLEL, TAG_FINAL]);
        self.current.current_executable_content = 0;
    }

    fn end_on_entry(&mut self) {
        let ct_id = self.current.current_executable_content;
        self.current.current_executable_content = 0;

        let state = self.get_current_state();
        // Assign the collected content to the on-exirt.
        state.onentry = ct_id;
    }

    fn start_on_exit(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_ON_EXIT, &[TAG_STATE, TAG_PARALLEL, TAG_FINAL]);
        self.current.current_executable_content = 0;
    }

    fn end_on_exit(&mut self) {
        let ct_id = self.current.current_executable_content;
        self.current.current_executable_content = 0;

        let state = self.get_current_state();
        // Assign the collected content to the on-exit.
        state.onexit = ct_id;
    }

    fn start_if(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_IF, &[TAG_ON_ENTRY, TAG_ON_EXIT, TAG_TRANSITION, TAG_FOR_EACH, TAG_IF]);
        todo!()
    }

    fn end_if(&mut self) {
        todo!()
    }

    fn start_else_if(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_ELSEIF, &[TAG_IF]);
        todo!()
    }

    fn start_else(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_ELSE, &[TAG_IF]);
        todo!()
    }

    fn start_raise(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_RAISE, &[TAG_TRANSITION, TAG_ON_EXIT, TAG_ON_ENTRY, TAG_IF, TAG_FOR_EACH]);
        todo!()
    }

    fn start_send(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_SEND, &[TAG_TRANSITION, TAG_ON_EXIT, TAG_ON_ENTRY, TAG_IF, TAG_FOR_EACH]);

        let mut sendParams = SendParameters::new();

        let mut event = attr.get(ATTR_EVENT);
        let mut eventexpr = attr.get(ATTR_EVENTEXPR);

        if event.is_some() {
            if eventexpr.is_some() {
                panic!("{}: attributes {} and {} must not occur both", TAG_SEND, ATTR_EVENT, ATTR_EVENTEXPR);
            }
            sendParams.event = event.unwrap().clone();
        } else if eventexpr.is_some() {
            sendParams.eventexpr = eventexpr.unwrap().clone();
        }

        let target = attr.get(ATTR_TARGET);
        let targetexpr = attr.get(ATTR_TARGETEXPR);
        if target.is_some() {
            if targetexpr.is_some() {
                panic!("{}: attributes {} and {} must not occur both", TAG_SEND, ATTR_TARGET, ATTR_TARGETEXPR);
            }
            sendParams.target = target.unwrap().clone();
        } else if targetexpr.is_some() {
            sendParams.targetexpr = targetexpr.unwrap().clone();
        }

        let typeS = attr.get(ATTR_TYPE);
        let typeexpr = attr.get(ATTR_TYPEEXPR);
        if typeS.is_some() {
            if typeexpr.is_some() {
                panic!("{}: attributes {} and {} must not occur both", TAG_SEND, ATTR_TYPE, ATTR_TYPEEXPR);
            }
            sendParams.typeS = typeS.unwrap().clone();
        } else if typeexpr.is_some() {
            sendParams.typeexpr = typeexpr.unwrap().clone();
        }

        let id = attr.get(ATTR_ID);
        let idlocation = attr.get(ATTR_IDLOCATION);
        if id.is_some() {
            if idlocation.is_some() {
                panic!("{}: attributes {} and {} must not occur both", TAG_SEND, ATTR_ID, ATTR_IDLOCATION);
            }
            sendParams.name = id.unwrap().clone();
        } else if idlocation.is_some() {
            sendParams.namelocation = idlocation.unwrap().clone();
        }

        let delay = attr.get(ATTR_DELAY);
        let delayExrp = attr.get(ATTR_DELAYEXPR);
        if delayExrp.is_some() {
            if delay.is_some() {
                panic!("{}: attributes {} and {} must not occur both", TAG_SEND, ATTR_DELAY, ATTR_DELAYEXPR);
            }
            sendParams.delayexrp = delayExrp.unwrap().clone();
        } else if delay.is_some() {
            sendParams.delay = delay.unwrap().clone();
        }

        let nameList = attr.get(ATTR_NAMELIST);
        if nameList.is_some() {
            sendParams.nameList = nameList.unwrap().clone();
        }
        self.add_executable_content(Box::new(sendParams));
    }

    fn start_content(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_CONTENT, &[TAG_SEND, TAG_INVOKE, TA_DONEDATA]);
        todo!()
    }

    fn end_content(&mut self) {
        todo!()
    }

    fn start_param(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_PARAM, &[TAG_SEND, TAG_INVOKE, TA_DONEDATA]);

        todo!()
    }


    fn start_log(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_LOG, &[TAG_TRANSITION, TAG_ON_EXIT, TAG_ON_ENTRY, TAG_IF, TAG_FOR_EACH]);
        let expr = attr.get(ATTR_EXPR);
        if expr.is_some() {
            self.add_executable_content(Box::new(Log::new(expr.unwrap().as_str())));
        }
    }

    fn start_assign(&mut self, attr: &AttributeMap) {
        self.verify_parent_tag(TAG_ASSIGN, &[TAG_TRANSITION, TAG_ON_EXIT, TAG_ON_ENTRY, TAG_IF, TAG_FOR_EACH]);
        todo!()
    }

    fn start_element(&mut self, reader: &Reader<Box<dyn BufRead>>, e: &BytesStart, txt: &mut Vec<String>) {
        let n = e.local_name();
        let name = str::from_utf8(n.as_ref()).unwrap();
        self.push(name);

        debug!("Start Element {}", name);

        let attr = &decode_attributes(&reader, &mut e.attributes());

        match name {
            TAG_INCLUDE => {
                self.include(attr);
            }
            TAG_SCXML => {
                if self.in_scxml {
                    panic!("Only one <{}> allowed", TAG_SCXML);
                }
                self.in_scxml = true;
                let datamodel = attr.get(ATTR_DATAMODEL);
                if datamodel.is_some() {
                    debug!(" scxml.datamodel = {}", datamodel.unwrap());
                    self.fsm.datamodel = crate::fsm::createDatamodel(datamodel.unwrap());
                }
                let version = attr.get(TAG_VERSION);
                if version.is_some() {
                    self.fsm.global().borrow_mut().version = version.unwrap().clone();
                    debug!(" scxml.version = {}", version.unwrap());
                }
                self.fsm.pseudo_root = self.get_or_create_state_with_attributes(&attr, false, 0);
                self.current.current_state = self.fsm.pseudo_root;
            }
            TAG_DATAMODEL => {
                self.start_datamodel();
            }
            TAG_DATA => {
                self.start_data(attr);
            }
            TAG_STATE => {
                self.start_state(attr);
            }
            TAG_PARALLEL => {
                self.start_parallel(attr);
            }
            TAG_FINAL => {
                self.start_final(attr);
            }
            TAG_HISTORY => {
                self.start_history(attr);
            }
            TAG_INITIAL => {
                self.start_initial();
            }
            TAG_TRANSITION => {
                self.start_transition(attr);
            }
            TAG_ON_ENTRY => {
                self.start_on_entry(attr);
            }
            TAG_ON_EXIT => {
                self.start_on_exit(attr);
            }
            TAG_SCRIPT => {
                txt.clear();
                self.start_script(attr);
            }
            TAG_RAISE => {
                self.start_raise(attr);
            }
            TAG_SEND => {
                self.start_send(attr);
            }
            TAG_PARAM => {
                self.start_param(attr);
            }
            TAG_CONTENT => {
                self.start_content(attr);
            }
            TAG_LOG => {
                self.start_log(attr);
            }
            TAG_ASSIGN => {
                self.start_assign(attr);
            }
            TAG_FOR_EACH => {
                self.start_for_each(attr);
            }
            TAG_CANCEL => {
                self.start_cancel(attr);
            }
            TAG_IF => {
                self.start_if(attr);
            }
            TAG_ELSE => {
                self.start_else(attr);
            }
            TAG_ELSEIF => {
                self.start_else_if(attr);
            }
            _ => {
                debug!("Ignored tag {}", name)
            }
        }
    }

    fn get_resolved_path(&self, ps: &String) -> PathBuf {
        let src = Path::new(ps).clone().to_owned();
        let parent = Path::new(&self.file).parent();
        match parent {
            Some(parent_path) => {
                let pp = parent_path.join(src);
                pp.to_owned()
            }
            None => {
                src.to_owned()
            }
        }
    }

    /// Handle a XInclude include element.
    /// See https://www.w3.org/TR/xinclude/
    /// Only parse="text" and "href" with a relative path are supported, also no "xpointer" etc.
    fn include(&mut self, attr: &AttributeMap) {
        let href = Self::get_required_attr(TAG_INCLUDE, TAG_HREF, attr);
        let parse = attr.get(ATTR_PARSE);
        if parse.is_none() || parse.unwrap().ne("text") {
            panic!("{}: only {}='text' is supported", TAG_INCLUDE, ATTR_PARSE)
        }
        let xpointer = attr.get(ATTR_XPOINTER);
        if xpointer.is_some() {
            panic!("{}: {} is not supported", TAG_INCLUDE, ATTR_XPOINTER)
        }

        let src = self.get_resolved_path(href);

        match File::open(src.clone()) {
            Ok(f) => {
                let org_file = self.file.clone();
                self.file = src.to_str().unwrap().to_string();
                match read_all_events(self, Box::new(BufReader::new(f))) {
                    Ok(t) => {}
                    Err(e) => {
                        panic!("Can't parse '{}' in <{}>. {}", src.to_str().unwrap(), TAG_INCLUDE, e);
                    }
                }
                self.file = org_file;
            }
            Err(e) => {
                panic!("Can't read '{}' in <{}>. {}", src.to_str().unwrap(), TAG_INCLUDE, e);
            }
        }
    }

    fn end_element(&mut self, name: &str, txt: &mut Vec<String>) {
        if !self.current.current_tag.eq(name) {
            panic!("Illegal end-tag {:?}, expected {:?}", &name, &self.current.current_tag);
        }
        debug!("End Element {}", name);
        match name {
            TAG_SCRIPT => {
                self.end_script(txt);
            }
            TAG_IF => {
                self.end_if();
            }
            TAG_TRANSITION => {
                self.end_transition();
            }
            TAG_ON_EXIT => {
                self.end_on_exit();
            }
            TAG_ON_ENTRY => {
                self.end_on_entry();
            }
            TAG_CONTENT => {
                self.end_content();
            }

            _ => {}
        }
        self.pop();
    }
}

/**
 * Decode attributes into a hash-map
 */
fn decode_attributes(reader: &Reader<Box<dyn BufRead>>, attr: &mut Attributes) -> AttributeMap {
    attr.map(|attr_result| {
        match attr_result {
            Ok(a) => {
                let key = reader.decoder().decode(a.key.as_ref());
                if key.is_err() {
                    panic!("unable to read attribute name {:?}, utf8 error {:?}", &a, key.err());
                }
                let value = a.decode_and_unescape_value(&reader);
                if value.is_err() {
                    panic!("unable to read attribute value  {:?}, utf8 error {:?}", &a, value.err());
                }
                (key.unwrap().to_string(), value.unwrap().to_string())
            }
            Err(err) => {
                panic!("unable to read key in DefaultSettings, err = {:?}", err);
            }
        }
    }).collect()
}

/// Reads the FSM from a XML file
pub fn read_from_xml_file(file: String) -> Result<Box<Fsm>, String> {
    match File::open(file.clone()) {
        Ok(f) => {
            read(Box::new(BufReader::new(f)), &file)
        }
        Err(e) => {
            Err(format!("Failed to read {}. {}", file, e))
        }
    }
}

/// Reads the FSM from a XML String
pub fn read_from_xml(xml: String) -> Result<Box<Fsm>, String> {
    let fake_file = "".to_string();
    read(Box::new(Cursor::new(xml)), &fake_file)
}

fn read(buf: Box<dyn BufRead>, f: &String) -> Result<Box<Fsm>, String> {
    let mut rs = ReaderState::new(f);
    let r = read_all_events(&mut rs, buf);
    match r {
        Ok(m) => {
            Ok(rs.fsm)
        }
        Err(e) => {
            Err(e)
        }
    }
}

fn read_all_events(rs: &mut ReaderState, buf: Box<dyn BufRead>) -> Result<&str, String> {
    debug!(">>> Reading {}", rs.file);

    let mut reader = Reader::from_reader(buf);
    reader.trim_text(true);
    let mut txt = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => {
                debug!("<<< {}", rs.file);
                return Err(format!("Error at position {}: {:?}", reader.buffer_position(), e));
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                rs.start_element(&mut reader, &e, &mut txt);
            }
            Ok(Event::End(e)) => {
                rs.end_element(str::from_utf8(e.local_name().as_ref()).unwrap(), &mut txt);
            }
            Ok(Event::Empty(e)) => {
                // Element without content.
                rs.start_element(&mut reader, &e, &mut txt);
                rs.end_element(str::from_utf8(e.local_name().as_ref()).unwrap(), &mut txt);
            }
            Ok(Event::Text(e)) => txt.push(e.unescape().unwrap().into_owned()),
            Ok(Event::Comment(e)) => debug!("Comment :{}", e.unescape().unwrap()),

            // Ignore other
            Ok(e) => debug!("Ignored SAX Event {:?}", e),
        }
        buf.clear();
    }
    debug!("<<< {}", rs.file);
    Ok("ok")
}

#[cfg(test)]
mod tests {
    use crate::fsm::Transition;

    #[test]
    #[should_panic]
    fn initial_attribute_should_panic() {
        crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'>\
    <initial><transition></transition></initial></state></scxml>".to_string());
    }

    #[test]
    #[should_panic]
    fn script_with_src_and_content_should_panic() {
        crate::reader::read_from_xml("<scxml initial='Main'><state id='Main'>\
    <initial><transition><script src='xml/example/script.js'>println();</script></transition></initial></state></scxml>".to_string());
    }

    #[test]
    fn script_with_src_should_load_file() {
        let r =
            crate::reader::read_from_xml("<scxml initial='Main'><state id='Main'>\
    <initial><transition><script src='xml/example/script.js'></script></transition></initial></state></scxml>".to_string());
        assert_eq!(r.is_ok(), true);

        let mut fsm = r.unwrap();

        for s in &fsm.states {
            println!("State #{} script: {}", s.id, s.script);
            if s.script != 0 {
                println!(" -> {:?}", fsm.global().borrow().executableContent.get(&s.script).unwrap());
            }
        }
    }


    #[test]
    fn initial_attribute() {
        crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'></state></scxml>".to_string());
    }

    #[test]
    fn wrong_end_tag_should_panic() {
        let r = crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'></parallel></scxml>".to_string());
        assert!(r.is_err(), "Shall result in error");
    }

    #[test]
    #[should_panic]
    fn wrong_parse_in_xinclude_should_panic() {
        crate::reader::read_from_xml(
            "<scxml><state><include href='xml/example/Test2Sub1.xml' parse='xml'/></state></scxml>".to_string());
    }

    #[test]
    #[should_panic]
    fn none_parse_in_xinclude_should_panic() {
        crate::reader::read_from_xml(
            "<scxml><state><include href='xml/example/Test2Sub1.xml'/></state></scxml>".to_string());
    }

    #[test]
    #[should_panic]
    fn xpointer_in_xinclude_should_panic() {
        crate::reader::read_from_xml(
            "<scxml><state><include href='xml/example/Test2Sub1.xml' parse='text' xpointer='#123'/></state></scxml>".to_string());
    }

    #[test]
    fn xinclude_should_read() {
        crate::reader::read_from_xml(
            "<scxml><state><include href='xml/example/Test2Sub1.xml' parse='text'/></state></scxml>".to_string());
    }

    #[test]
    #[should_panic]
    fn wrong_transition_type_should_panic() {
        crate::reader::read_from_xml(
            "<scxml><state><transition type='bla'></transition></state></scxml>".to_string());
    }

    #[test]
    fn transition_type_internal() {
        crate::reader::read_from_xml(
            "<scxml><state><transition type='internal'></transition></state></scxml>".to_string());
    }

    #[test]
    fn transition_type_external() {
        crate::reader::read_from_xml(
            "<scxml><state><transition type='external'></transition></state></scxml>".to_string());
    }
}