# Finite State Machine Implementation in RUST

According to W3C Recommendations

See https://www.w3.org/TR/scxml/

To-Do:

+ Implement XML-Reading _(ongoing)_
+ Implement the model (fsm, states etc.) _(mostly finished)_
+ <s>Implement Datastructures needed (Queue etc.)-- _(finished)_</s>
+ Implement w3c algorithm (mostly finished).
+ Implement ECMAScript Datamodel _(ongoing)_
+ Design concept for "invoke"
    + Life-cycle control of threads / processes.
    + Communication: See Cancel-methods. <br>
      We can use events via external-queue, but spec doesn't force this.
    + In the Architecture below "caller_sender" and "caller_invoke_id"
      are added for supporting notification of invoker that triggered execution of this fsm.

## Architecture

### Overview

```mermaid
erDiagram
    Fsm {
    }

    reader {
    }
   

    Datamodel {
    }

    XML }o--|| reader : parse
    XML {
    }

    reader ||--o{ Fsm : creates   

    Fsm ||--|| Datamodel : owns
    
    ECMAScript ||--|| Datamodel : implements
    NullDatamodel ||--|| Datamodel : implements
 
```

### Basic class diagram

```mermaid
classDiagram
    Fsm *-- Datamodel
    Fsm *-- BlockingQueue: external queue
    Fsm *-- Queue: internal queue

    class Fsm{
      +Datamodel datamodel
      +Queue internalQueue
      +BlockingQueue externalQueue
      +Tracer tracer
      +StateId pseudo_root
      +Vec~StateId~ states
      +HashMap executableContent
      +HashMap transitions
      +DataStore data
      +Sender caller_sender
      +InvokeId caller_invoke_id
      
      +addAncestorStatesToEnter(State,ancestor,statesToEnter,statesForDefaultEntry,defaultHistoryContent)
      +addDescendantStatesToEnter(State,statesToEnter,statesForDefaultEntry, defaultHistoryContent)
      +applyFinalize(InvokeId, Event)
      +cancelInvoke(InvokeId)
      +computeExitSet(transitions) OrderedSet~StateId~
      +conditionMatch(Transition) boolean
      +enterStates(enabledTransitions)
      +exitInterpreter()
      +exitStates(enabledTransitions)
      +findLCCA(stateList) State
      +getChildStates(State) List~State~
      +getEffectiveTargetStates(Transition) OrderedSet~State~
      +getProperAncestors(State1, State2) OrderedSet~StateId~
      +getTransitionDomain(Transition) State
      +interpret()
      +invoke(Invoke)
      +isCancelEvent(Event) boolean
      +isCompoundState(State) boolean
      +isCompoundStateOrScxmlElement(State) boolean
      +isDescendant(State1, State2) boolean
      +isHistoryState(State) boolean
      +isInFinalState(state) boolean
      +mainEventLoop()
      +microstep(enabledTransitions)
      +nameMatch(Vec<Event>, String) boolean
      +removeConflictingTransitions(enabledTransitions) OrderedSet~Transition~
      +returnDoneEvent(DoneData)
      +selectEventlessTransitions() OrderedSet~Transition~
      +selectTransitions(Event) OrderedSet~Transition~
   }
    
    Fsm "1" *-- "1..n" State
    State "1" *-- "0..n" State: Composition
    State  *-- "0..n" Transition: Outgoing
    Transition  --> "0..n" State : Targets
    
    class State {
    
    }

    class Transition {
      +event
      +cond: BooleanExpression
      +target: StateId[]
      +type: "internal" or "external"
      +ExecutableContent
    }

    <<Interface>> Datamodel
    class Datamodel{    
     +get_name() : String
     +global() GlobalData

     +initializeDataModel(Data)

     +set(name, Data);
     +get(name): Data;

     +execute(Script): String;
     +executeCondition(BooleanExpression): boolean
     
     +log(String)
    }

    class ECMAScriptDatamodel {
     +data: DataStore
     +context_id: u32
    }

    class ECMAScriptContext {
      +global_data: GlobalData
      +context: boa_engine::Context
    }

    
    class context_map{
    }
    
    context_map "1" *-- "*" ECMAScriptContext

    class BlockingQueue{
    }
    
    ECMAScriptDatamodel ..|> Datamodel
    ECMAScriptDatamodel --> context_map: context_id
    
    NullDatamodel ..|> Datamodel
    
    class reader {
     +read_from_xml(xml) Fsm
     +read_from_xml_file(path) Fsm
    }
      
    reader --> Fsm : Creates 
```

The Fsm implements the methods described in the W3C Recommendation. The main loop is executed in a work-thread. The application sends events via a "BlockingQueue" (technical a
Channel). All scripts and conditional expressions are executed by the Datamodel. W3C defines that implementations may freely choose the script language to support. The mandatory
Null-Datamodel is a dummy implementation with minimal functionality.

In this implementation ECMAScript is integrated (if feature "ECMAScript" is active). It uses [boa-engine](https://boajs.dev/). Boa claims to have a ~75% conformance to the
Standard. So don't expect to have 100% JavaScript here.

You can check the requirements for ECMA in SCXML [here](https://www.w3.org/TR/scxml/#ecma-profile)




