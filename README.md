# Finite State Machine (FSM) Implementation in Rust

[![Test Status](https://github.com/BWeng20/rFSM/actions/workflows/rust.yml/badge.svg)](https://github.com/BWeng20/rFSM/actions/workflows/rust.yml) [![Rust-Slippy](https://github.com/BWeng20/rFSM/actions/workflows/rust-clippy.yml/badge.svg)](https://github.com/BWeng20/rFSM/actions/workflows/rust-clippy.yml)

According to W3C Recommendations, reading and executing State Chart XML (SCXML).

See https://www.w3.org/TR/scxml/

Currently, the project passed all 160 of the mandatory automated tests from the W3C test-suite.<br/>
See [Test Readme](test/w3c/README.md) and the [Test Report](test/w3c/REPORT.MD).

## To-Dos:

+ Implement BasicHttpEvent I/O Processor _(ongoing, see [basic_http_event_io_processor.rs](src/basic_http_event_io_processor.rs))_.
+ Design a concept for dynamic binding of business-logic to the FSM (to do real things). 


> [!NOTE]
> ### Not conformant or not implemented features of the W3C recommendation
> 
> + XML inside &lt;content> is not handled according to _[content_and_namespaces](doc/W3C_SCXML_2024_07_13/index.html#content_and_namespaces)_. The content inside &lt;content> is not
>  interpreted and send to the receiver unmodified.
> + BasicHTTP Event I/O processor doesn't set the '_event.raw' member, that is needed for optional 
>   tests 178, 509, 519, 520 and 534.

## SW Design

See [SW Design](SW_Design.md)

## Crate Features

| Name                      | Description                                                | Related crates                                            |
|---------------------------|------------------------------------------------------------|-----------------------------------------------------------|
| ECMAScript                | Adds an EMCAScript datamodel implementation.               | boa_engine                                                |
| EnvLog                    | The crate "env_log" is used as "log" implementation.       | env_log                                                   |
| BasicHttpEventIOProcessor | Adds an implementation of BasicHttpEventIOProcessor        | hyper, http-body-util, hyper-util, tokio, form_urlencoded |
| json-config               | The test tool can read configurations in JSON.             | serde_json                                                |
| yaml-config               | The test tool can read configurations in YAML.             | yaml-rust                                                 |
| Trace_Method              | Enables tracing of methods calls in the FSM.               |                                                           |
| Trace_State               | Enables tracing of state changes in the FSM.               |                                                           |
| Trace_Event               | Enables tracing of events in the FSM.                      |                                                           |
| Xml                       | Enables reading SCXML (xml) files. Currently no other way. | quick-xml, reqwest                                        |
| Debug_Reader              | Enables debug output in the SCXML reader (a lot).          |                                                           |

The trace options <i>Trace_xxx</i> still needed to be activated during runtime by settings the trace-mode.
If none of the <i>Trace_xxx</i> features are used, "Tracer" module is completely removed.

## How To Use

FSMs normally are used embedded inside other software to control some state-full workflow.<br/> 
The operations of this workflow are triggered from the transitions or the states-handlers.  
To add such operations different approaches exits. In hard-codes FSMs methods-calls are directly
linked to the transition or states during compile-time.<br/>

This implementation loads FSMs during runtime, so the binding needs to be dynamically.<br/> 
The API for this binding currently doesn't exist (a todo).

The only practical use (currently) is to test SCXML documents (see [Manual Testing](SW_Design.md#manual-tests)) 
