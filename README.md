# Finite State Machine (FSM) Implementation in Rust

[![Test Status](https://github.com/BWeng20/rFSM/actions/workflows/rust.yml/badge.svg)](https://github.com/BWeng20/rFSM/actions/workflows/rust.yml) [![Rust-Slippy](https://github.com/BWeng20/rFSM/actions/workflows/rust-clippy.yml/badge.svg)](https://github.com/BWeng20/rFSM/actions/workflows/rust-clippy.yml)

![logo](logo.svg)

This project implements an embeddable and extensible Harel Statechart interpreter.  
Multiple state machines can be loaded and executed in parallel at runtime.  
Different FSMs can communicate internally or externally via events.

A datamodel can be used to maintain data and execute business logic.

## Software Design

This crate implements a _State Chart XML_ (SCXML) interpreter according to the [W3C Recommendation](https://www.w3.org/TR/scxml/).  
For details, see [SW Design](SW_Design.md).

## Main Crate Features

The main functional feature switches of the project:

| Name                      | Description                                                                                                     | Related Crates        | Impact on Size<br/>of Release Build [^1] |
|---------------------------|-----------------------------------------------------------------------------------------------------------------|------------------------|-------------------------------------------|
| ECMAScriptModel           | Adds an ECMAScript datamodel implementation.                                                                    | boa_engine             | + ~ 10.25 MiB                              |
| xml                       | Enables reading SCXML (XML) files.                                                                              | quick-xml, ureq, url   | + ~ 2.07 MiB                               |
| RfsmExpressionModel       | Adds a datamodel implementation based on the internal Expression Engine.                                        |                        | + ~ 0.09 MiB                               |
| serializer                | Support for reading/writing FSMs in a binary property format – as an alternative to XML.                        |                        | + ~ 0.1 MiB                                |
| BasicHttpEventIOProcessor | Adds an implementation of BasicHttpEventIOProcessor.                                                            | rocket, ureq           | + ~ 4.97 MiB                               |
| json-config               | The test tool can read configurations in JSON.                                                                  | serde_json             | + ~ 0.003 MiB                              |
| yaml-config               | The test tool can read configurations in YAML.                                                                  | yaml-rust              | – ~ 0.001 MiB                              |
| EnvLog                    | Uses the `env_log` crate as a logging backend. Otherwise, `std::println` is used.                               | env_log                | + ~ 1.21 MiB                               |
| ThriftTrace               | Enables Thrift Trace Server.                                                                                      |                        | _– not finished –_                         |

[^1]: Features share dependencies, so the resulting binary size of combined features is smaller than the sum of individual features.

A minimal feature set for an MVP:

+ `RfsmExpressionModel` — _Minimalistic datamodel using an expression language._
+ `serializer` — _Reads FSMs from binary `.rfsm` files._

This results in a minimal release binary size of approximately **1.4 MiB**.

## Extended Crate Features (for Test and Debugging)

| Name         | Description                                          | Related Crates | Impact on Size<br/>of Release Build |
|--------------|------------------------------------------------------|----------------|-------------------------------------|
| Trace_Method | Enables tracing of method calls in the FSM.         |                | [^2]                                |
| Trace_State  | Enables tracing of state changes in the FSM.        |                | [^2]                                |
| Trace_Event  | Enables tracing of events in the FSM.               |                | [^2]                                |
| Debug_Reader | Enables extensive debug output for the SCXML reader.|                | _don't use it!_                     |
| Debug        | Enables additional internal debug output.           |                | _don't use it!_                     |

The trace features `Trace_*` must also be activated at runtime via the trace mode setting.  
If none of the trace features are enabled, the tracer module is completely excluded from the build.

[^2]: Any trace feature increases the release binary size by ~0.03 MiB.

## About SCXML Files

SCXML is an XML format that requires a parser. FSMs typically remain small, but XML parsing is costly.  
As an alternative, this crate provides a platform-independent binary format with zero dependencies and a small codebase — and it’s much faster.  
To use this format, convert your SCXML files using the `scxml_to_fsm` tool.  
This binary is only built if the `xml` feature is enabled, so it must be compiled separately.  
Then you can build the main FSM crate without `xml`.

## Tracer

The Tracer module can be used to monitor the FSM.<br/>

The default-tracer simply prints the traced actions. If the Remote-Trace-Server-Feature is enabled, a Server is 
started that can be used by remote-clients (_to be done_).

The Tracer has different flags to control what is traced, see Enum [TraceMode](src/tracer.rs) for details.

## How To Use

FSMs normally are used embedded inside other software to control some state-full workflow.<br/> 
The transitions or the states trigger operations in the business workflow .  
To bind the business logic to a state machine different approaches exits. In hard-coded FSMs methods-calls are directly
linked to the transitions or state-actions during compile-time. Most state machine frameworks work this way.<br/>

This crate loads FSMs during runtime, so the binding to the FSM needs to be dynamical.<br/> 
SCXML defines a _Datamodel_ for this purpose.

### Datamodel

For details about the Datamodel-concept in SCXML see the W3C documentation. This lib provides some implementations of 
the Datamodel, but you can implement your own models as well.<br/>
The Datamodel in SCXML is responsible to execute scripts and formulas. Custom business logic
can be implemented this way.<br/>
For some huge project, it may be feasible to implement the Datamodel-trait with some optimized way to trigger the 
business-functionality.<br/>

For a simpler approach (without implement a full Datamodel), see "Custom Actions" below.
You can also find some examples inside folder "examples".<br/>
Each Datamodel has a unique identifier, that can be selected in the SCXML-source, so you can provide multiple model-implementation in
one binary and use them in parallel in different FSMs.

To add new data-models, use function `rufsm::fsm::register_datamodel`.

### Provided Datamodel Implementations

+ EMCAScript-Datamodel, use `datamodel="ecmascript"`. Available if feature _"ECMAScriptModel"_ is turned on.
+ The Null-Datamodel, use `datamodel="null"`
+ Internal Expression Engine Datamodel, use `datamodel="rfsm-expression"`. Available if feature _"RfsmExpressionModel"_ is turned on.

As the EMCAScript-Datamodel is based on boa-engine, it results in a huge binary. 
If you need only basic logic in your scripts, use "rfsm-expression" instead.

For details see the [Expression-Engine-Readme](src/expression_engine/README.md).

### Custom Actions

You can use the trait "Action" to add custom functions to the FSM. See the Examples for a How-To.
Each FSM instance can have a different set of Actions. Action are inherited by child-sessions.

If using ECMAScript- or RfsmExpressions-Datamodel, these actions can be called like normal methods. 
Arguments and return values will be converted from and to JavaScript/Data-values. See enum "Data" for supported data-types.

Actions have full access to the data and states of the FSM.

## Tests

For basic functions the project contains several unit tests. The current status of these tests can be seen on the
repository start page.

More complex tests are done by test scripts that executes SCXML-files provided by the W3C.</br>
Currently, the project passed all 160 of the mandatory automated tests from the W3C test-suite.
For the detailed test process see [Test Readme](test/w3c/README.md) and for the results [Test Report](test/w3c/REPORT.MD).

## To-Dos:

+ Implement the Trace-Server to bind IDEs to the runtime.
+ Design and implement some meaning full I/O-processor as the "basic-html" is not usable for production.
  + MQTT?
  + REST-API?
  + Some fast Domain-Socket (linux) / Pipe (Windows) based I/O-Processor? 



> [!NOTE]
> ### Not conformant or not implemented features of the W3C recommendation
>
> + XML inside &lt;content> is not handled according to _[content_and_namespaces](doc/W3C_SCXML_2024_07_13/index.html#content_and_namespaces)_. The content inside &lt;content> is not
    >  interpreted and send to the receiver unmodified.
> + BasicHTTP Event I/O processor doesn't set the '_event.raw' member, that is needed for optional
    >   tests 178, 509, 519, 520 and 534.
