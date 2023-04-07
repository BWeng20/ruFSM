extern crate core;

use std::{env, io, process, thread, time};
use std::io::{stdout, Write};
use std::str::FromStr;
use std::sync::mpsc::Sender;

use crate::fsm::{Event, EventType, Trace};

mod reader;
mod fsm;
mod executable_content;

#[cfg(feature = "ECMAScript")]
mod ecma_script_datamodel;


fn handleTrace(sender: &mut Sender<Box<Event>>, opt: &str, enable: bool) {
    match Trace::from_str(opt) {
        Ok(t) => {
            let event = Box::new(Event::trace(t, enable));
            match sender.send(event) {
                Ok(_r) => {
                    // ok
                }
                Err(e) => {
                    eprintln!("Error sending trace event: {}", e);
                }
            }
        }
        Err(_e) => {
            println!("Unknown trace option. Use one of:\n methods\n states\n events\n arguments\n results\n all\n");
        }
    }
}

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
            let (_thread_handle, mut sender) = fsm::start_fsm(sm);

            let mut line = String::new();
            let stdin = io::stdin();
            let empty_str = "".to_string();

            loop {
                thread::sleep(time::Duration::from_millis(200));
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
                            handleTrace(&mut sender, &line_lc[5..], true);
                        } else if line_lc.starts_with("troff") && line_lc.len() > 6 {
                            handleTrace(&mut sender, &line_lc[6..], false);
                        } else if !line_lc.eq("help") && !line.is_empty() {
                            let event = Box::new(Event {
                                name: line.clone(),
                                etype: EventType::platform,
                                sendid: 0,
                                origin: empty_str.clone(),
                                origintype: empty_str.clone(),
                                invokeid: 1,
                                data: None,
                            });
                            match sender.send(event) {
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
                            println!(r#"Usage:
  Use 'Tron <flag>' or 'Troff <flag>' to control trace-levels.
  E.g. enter: tron all
  To send events, type the name of the event and press enter.
  Remind that Events are case sensitive.
  To print this information enter 'help' or an empty line.
  "#);
                        }
                    }

                    Err(e) => {
                        eprintln!("Error: {}. aborting...", e);
                        process::exit(-1);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open {} error {}", args[0], e);
            process::exit(2);
        }
    }
}


