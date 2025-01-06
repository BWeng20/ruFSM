# Finite State Machine (FSM) Implementation in Rust

[![Test Status](https://github.com/BWeng20/rFSM/actions/workflows/rust.yml/badge.svg)](https://github.com/BWeng20/rFSM/actions/workflows/rust.yml) [![Rust-Slippy](https://github.com/BWeng20/rFSM/actions/workflows/rust-clippy.yml/badge.svg)](https://github.com/BWeng20/rFSM/actions/workflows/rust-clippy.yml)

According to W3C Recommendations, reading and executing State Chart XML (SCXML).

See https://www.w3.org/TR/scxml/

Currently, the project passed all 160 of the mandatory automated tests from the W3C test-suite.<br/>
See [Test Readme](test/w3c/README.md) and the [Test Report](test/w3c/REPORT.MD).

## To-Dos:

+ Implement BasicHttpEvent I/O Processor _(ongoing, see [basic_http_event_io_processor.rs](src/basic_http_event_io_processor.rs))_.
+ Implement the Trace-Server to bind IDEs to runtime.


> [!NOTE]
> ### Not conformant or not implemented features of the W3C recommendation
> 
> + XML inside &lt;content> is not handled according to _[content_and_namespaces](doc/W3C_SCXML_2024_07_13/index.html#content_and_namespaces)_. The content inside &lt;content> is not
>  interpreted and send to the receiver unmodified.
> + BasicHTTP Event I/O processor doesn't set the '_event.raw' member, that is needed for optional 
>   tests 178, 509, 519, 520 and 534.

## SW Design

See [SW Design](SW_Design.md)

## Main Crate Features

| Name                      | Description                                                                                                     | Related crates                                            | Impact on Size<br/>of Release Build |
|---------------------------|-----------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------|-------------------------------------|
| ECMAScript                | Adds an EMCAScript datamodel implementation.                                                                    | boa_engine                                                | +&#160;~&#160;10.25&#160;MiB        |
| xml                       | Enables reading SCXML (xml) files.                                                                              | quick-xml, reqwest                                        | +&#160;~&#160;2,5&#160;MiB          |
| RfsmExpressionModel       | Adds a datamodel implementation based on the internal Expression-Engine.                                        |                                                           | +&#160;~&#160;0.09&#160;MiB         |
| serializer                | Support for reading/writing FSMs in a property binary format - as alternative to xml.                           |                                                           | +&#160;~&#160;0.1 MiB               |
| BasicHttpEventIOProcessor | Adds an implementation of BasicHttpEventIOProcessor                                                             | hyper, http-body-util, hyper-util, tokio, form_urlencoded | _- not finished -_                  |
| json-config               | The test tool can read configurations in JSON.                                                                  | serde_json                                                | +&#160;~&#160;0.003&#160;MiB        |
| yaml-config               | The test tool can read configurations in YAML.                                                                  | yaml-rust                                                 | -&#160;~&#160;0.001&#160;MiB        |
| EnvLog                    | The crate "env_log" is used as "log" implementation and for internal logging. Otherwise `std::println` is used. | env_log                                                   | +&#160;~&#160;1.21&#160;MiB         |
| TraceServer               | Enables Remote Trace Server.                                                                                    |                                                           | _- not finished -_                  |

The minimal feature set for a MVP is 
 + json-config - _used by the test-application_.
 + RfsmExpressionModel - _Datamodel for expression-language_
 + serializer - _Reads binary `rfsm` files_.

The rfsm files can be converted offline from SCXML by `scxml_to_fsm`.<br/>
This leads to a release binary of ~1.4 MiB in size. 
 

## Extended Crate Features (for test and debugging)


| Name                      | Description                                                              | Related crates                                            | Impact on Size<br/>of Release Build |
|---------------------------|--------------------------------------------------------------------------|-----------------------------------------------------------|-------------------------------------|
| Trace_Method              | Enables tracing of methods calls in the FSM.                             |                                                           | [^1]                                |
| Trace_State               | Enables tracing of state changes in the FSM.                             |                                                           | [^1]                                |
| Trace_Event               | Enables tracing of events in the FSM.                                    |                                                           | [^1]                                |
| Debug_Reader              | Enables debug output in the SCXML reader (a lot).                        |                                                           | _don't use it!_                     |
| Debug                     | Enables additional debug (to fnd errors).                                |                                                           | _don't use it!_                     |


The trace options <i>Trace_xxx</i> still needed to be activated during runtime by settings the trace-mode.
If none of the <i>Trace_xxx</i> features are used, "Tracer" module is completely removed.

[^1]: If any of the "Trace_XXXX" features is turned on, the release build will get ~ 0.03 MiB larger. 

## Tracer

The Tracer module can be used to monitor events and transitions.<br/>

The default-tracer simply prints the traced actions. If the Remote-Trace-Server is activated, the default-tracer is 
replaced by a tracer that communicates via the Remote-Trace-Server with some remote-client.   

## How To Use

FSMs normally are used embedded inside other software to control some state-full workflow.<br/> 
The operations of this workflow are triggered from the transitions or the states-handlers.  
To add such operations different approaches exits. In hard-codes FSMs methods-calls are directly
linked to the transitions or states during compile-time.<br/>

This implementation loads FSMs during runtime, so the binding needs to be dynamically.<br/> 

To bind business logic, SCXML defines a _Datamodel_.

### Datamodel

For details about the Datamodel-concept in SCXML see the W3C documentation. This lib provides some implementations of 
the Datamodel, but you can implement your own models as well.<br/>
The Datamodel in SCXML is responsible to execute code and expressions. Custom business logic
can be implemented this way.<br/>
For some huge project, it may be feasible to implement the Datamodel-trait with some optimized way to trigger the 
business-functionality.<br/>

For a simpler approach (without implement a full Datamodel), see "Custom Actions" below.
You can also find some examples inside folder "examples".<br/>
The Datamodel is selected in the SCXML, so you can provide multiple model-implementation in
one binary.

To add new data-models, use function `rfsm::fsm::register_datamodel`.

### Provided Datamodel Implementations

+ EMCAScript-Datamodel, use `datamodel="ecmascript"`. Available if feature _"ECMAScript"_ is turned on.
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
