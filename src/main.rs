extern crate core;

use std::{env, io, process, thread, time};
use std::io::{stdout, Write};

use crate::fsm::{Event, EventType, Trace};

mod reader;
mod fsm;
mod executable_content;

#[cfg(feature = "ECMAScript")]
mod ecma_script_datamodel;

/// Loads the specified FSM and prompts for Events.
fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Missing argument. Please specify a scxml file");
        process::exit(1);
    }

    println!("Loading FSM from {}", args[1]);

    match reader::read_from_xml_file(args[1].clone()) {
        Ok(mut sm) => {
            sm.tracer.enableTrace(Trace::ALL);
            let (threadHandle, sender) = fsm::start_fsm(sm);

            let mut line = String::new();
            let stdin = io::stdin();
            let emptyStr = "".to_string();

            loop {
                thread::sleep(time::Duration::from_millis(200));
                print!("\nEnter Event >>");
                stdout().flush();
                match stdin.read_line(&mut line) {
                    Ok(S) => {
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        }
                        sender.send(Box::new(Event {
                            name: line.clone(),
                            etype: EventType::platform,
                            sendid: 0,
                            origin: emptyStr.clone(),
                            origintype: emptyStr.clone(),
                            invokeid: 1,
                            data: None,
                        }));
                    }

                    Err(e) => {
                        eprintln!("Error: {}. aborting...", e);
                        process::exit(-1);
                    }
                }
            }
            threadHandle.join();
        }
        Err(e) => {
            eprintln!("Failed to open {} error {}", args[0], e);
            process::exit(2);
        }
    }
}


