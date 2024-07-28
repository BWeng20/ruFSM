//! Demonstration and Test application.
//! Usage:
//!    rfsm \<scxml-file\> \[-trace flag\]
extern crate core;

use std::io::{stdout, Write};
use std::{io, process, thread, time};

use rfsm::fsm::{Event, EventType};
use rfsm::fsm_executor::FsmExecutor;
#[cfg(feature = "Trace")]
use rfsm::handle_trace;
use rfsm::scxml_reader::INCLUDE_PATH_ARGUMENT_OPTION;
#[cfg(feature = "Trace")]
use rfsm::tracer::{TraceMode, TRACE_ARGUMENT_OPTION};

/// Loads the specified FSM and prompts for Events.
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    #[cfg(feature = "EnvLog")]
    env_logger::init();

    let (named_opt, final_args) = rfsm::get_arguments(&[
        #[cfg(feature = "Trace")]
        &TRACE_ARGUMENT_OPTION,
        #[cfg(feature = "xml")]
        &INCLUDE_PATH_ARGUMENT_OPTION,
    ]);

    #[cfg(feature = "Trace")]
    let trace = TraceMode::from_arguments(&named_opt);

    if final_args.len() < 1 {
        println!("Missing argument. Please specify one or more scxml file");
        process::exit(1);
    }

    let mut executor = FsmExecutor::new_with_io_processor().await;
    #[cfg(feature = "xml")]
    executor.set_include_paths_from_arguments(&named_opt);

    let mut session = executor
        .execute(
            final_args[0].as_str(),
            #[cfg(feature = "Trace")]
            trace,
        )
        .unwrap();

    for fi in 1..final_args.len() {
        let _ = executor
            .execute(
                final_args[fi].as_str(),
                #[cfg(feature = "Trace")]
                trace,
            )
            .unwrap();
    }

    let mut line = String::new();
    let stdin = io::stdin();
    let empty_str = "".to_string();

    loop {
        // let the FSM some time to process.
        // only needed to ensure that the prompt will be printed after normal FSM output.
        thread::sleep(time::Duration::from_millis(200));

        // If FSM was reached final state(s) the worker thread will be finished.
        match &session.session_thread {
            None => {}
            Some(thread) => {
                if thread.is_finished() {
                    println!("\nSM finished!");
                    executor.shutdown();
                    // TODO: dump data from the "finish"
                    break;
                }
            }
        }
        print!("\nEnter Event >>");
        match stdout().flush() {
            _ => {}
        }
        line.clear();
        match stdin.read_line(&mut line) {
            Ok(_s) => {
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                let line_lc = line.to_lowercase();
                if line_lc.starts_with("tron") && line.len() > 5 {
                    #[cfg(feature = "Trace")]
                    handle_trace(&mut session.sender, &line_lc[5..], true);
                } else if line_lc.starts_with("troff") && line_lc.len() > 6 {
                    #[cfg(feature = "Trace")]
                    handle_trace(&mut session.sender, &line_lc[6..], false);
                } else if !line_lc.eq("help") && !line.is_empty() {
                    let event = Box::new(Event {
                        name: line.clone(),
                        etype: EventType::platform,
                        sendid: empty_str.clone(),
                        origin: None,
                        origin_type: None,
                        invoke_id: Some(1.to_string()),
                        param_values: None,
                        content: None,
                    });
                    match session.sender.send(event) {
                        Ok(_r) => {
                            // ok
                        }
                        Err(e) => {
                            eprintln!("Error sending event: {}", e);
                            eprintln!("Aborting...");
                            process::exit(-2);
                        }
                    }
                } else {
                    println!(
                        r#"Usage:
Use 'Tron <flag>' or 'Troff <flag>' to control trace-levels.
E.g. enter: tron all
To send events, type the name of the event and press enter.
Remind that Events are case sensitive.
To print this information enter 'help' or an empty line.
"#
                    );
                }
            }

            Err(e) => {
                eprintln!("Error: {}. aborting...", e);
                process::exit(-1);
            }
        }
    }
}
