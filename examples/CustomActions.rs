use log::{debug, error};
use rfsm::actions::{Action, ActionWrapper};
use rfsm::datamodel::{Data, GlobalDataArc};
use rfsm::fsm_executor::FsmExecutor;
use rfsm::init_logging;
use std::process::exit;

#[cfg(feature = "Trace")]
use rfsm::tracer::TraceMode;

#[derive(Clone)]
pub struct MyAction {}

impl Action for MyAction {
    fn execute(&self, arguments: &[Data], _global: &GlobalDataArc) -> Result<Data, String> {
        let mut i = 0;
        println!("MyAction called with {} arguments:", arguments.len());
        for data in arguments {
            i += 1;
            println!("\t{}: {}", i, data)
        }
        Ok(Data::Boolean(true))
    }

    fn get_copy(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
}

/// The FSM needs tokio.
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    debug!(
        r#"Action Example
-----------------------------------------
The FSM will call two custom actions from different places.
These actions can be called at any element that containt expressions or executable content.
"#
    );

    // In this example we don't need any io-processor.
    // Otherwise:
    // let mut executor = FsmExecutor::new_with_io_processor().await;
    let mut executor = FsmExecutor::new_without_io_processor();
    let mut actions = ActionWrapper::new();

    // We register the same function with different names.
    let my_action = MyAction {};
    actions.add_action("myEnterAction", my_action.get_copy());
    actions.add_action("myLeaveAction", Box::new(my_action));

    let session;

    match executor.execute(
        "examples/CustomActions.scxml",
        actions,
        #[cfg(feature = "Trace")]
        TraceMode::ALL,
    ) {
        Ok(s) => {
            session = s;
        }
        Err(err) => {
            error!("Failed to execute: {}", err);
            exit(1);
        }
    };
    let _ = session.session_thread.unwrap().join();
    exit(0)
}
