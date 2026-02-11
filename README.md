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
| TraceServer               | Enables Remote Trace Server.                                                                                    |                        | _– not finished –_                         |

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

The `Tracer` module can be used to monitor FSM execution.

The default tracer prints traced actions. If the `TraceServer` feature is enabled, a remote server is started (work in progress).

The tracer has various flags to control what is being traced — see the `TraceMode` enum in [`src/tracer.rs`](src/tracer.rs).

## How to Use

FSMs are typically embedded in software to control stateful workflows.  
Transitions or states trigger operations in the business logic.  
In hard-coded FSMs, methods are directly bound to states or transitions at compile time (which is what most FSM frameworks do).

Since this crate loads FSMs at runtime, the bindings need to be dynamic.  
SCXML provides a _datamodel_ abstraction for this purpose.

### Datamodel

See the W3C documentation for more details on the SCXML datamodel concept.  
This library provides several implementations — or you can define your own.  
The datamodel executes scripts and expressions and may serve as an interface to business logic.

For large projects, you might consider implementing the `Datamodel` trait to trigger business functionality efficiently.

If you don’t want to implement a full datamodel, see the **Custom Actions** section below.  
Examples can be found in the `examples/` directory.  

Each datamodel has a unique ID that can be referenced in the SCXML source.  
This enables multiple data models to coexist in a single binary and be used in parallel.

To register new data models, call:
```rust
rufsm::fsm::register_datamodel
```

### Built-in Data Models

+ ECMAScript datamodel, use `datamodel="ecmascript"` (requires feature `ECMAScriptModel` feature).
+ The Null-Datamodel, use `datamodel="null"`
+ Internal Expression Engine Datamodel, use `datamodel="rfsm-expression"` (requires feature `RfsmExpressionModel`).

Note: The ECMAScript engine depends on `boa-engine`, which substantially increases binary size. 
If you only need basic expressions, use `rfsm-expression`.

More info: [Expression-Engine-Readme](src/expression_engine/README.md).

### Custom Actions

You can define custom logic by implementing the `Action` trait. 
Each FSM instance can register its own actions; these are inherited by child sessions.

In the ECMAScript or `rfsm-expression` data models, these actions can be called like normal functions. 
Parameters and return values are converted automatically — see the `Data` enum for supported types.

Custom actions have full access to FSM data and state.

## Tests

For basic functions the project contains several unit tests. The current status of these tests can be seen on the
repository start page.

More complex tests are done by test scripts that executes SCXML-files provided by the W3C.<br/>
Currently, the project passed all 160 of the mandatory automated tests from the W3C test-suite.
For the details, see [W3C Test README](test/w3c/README.md) and [W3C Test Report](test/w3c/REPORT.MD).

## To-Dos:

+ Implement the Trace-Server to support IDE plugins.
+ Design and implement production-ready I/O processors (beyond the basic HTTP), e.g.:
  + MQTT?
  + REST API?
  + Fast IPC: Domain socket (Linux) or Named pipe (Windows)? 


> [!NOTE]
> ### Not conformant or not implemented features of the W3C recommendation
>
> + XML inside &lt;content> is not handled according to _[content_and_namespaces](doc/W3C_SCXML_2024_07_13/index.html#content_and_namespaces)_. The content inside &lt;content&gt; is not
>   interpreted and send to the receiver unmodified.
> + The BasicHTTP Event I/O processor does not populate the '_event.raw' member, which is required by W3C optional tests 178, 509, 519, 520 and 534.
