extern crate core;

use std::{env, process};

use crate::fsm::{Event, EventType, Trace};

mod reader;
mod fsm;

#[cfg(feature = "ECMAScript")]
mod ecma_script_datamodel;

fn main() {
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
            threadHandle.join();
        }
        Err(e) => {
            eprintln!("Failed to open {} error {}", args[0], e);
            process::exit(2);
        }
    }
}


