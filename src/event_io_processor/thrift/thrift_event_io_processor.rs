//! I/O Processor implementation using Thrift

use std::fmt::{Debug, Formatter};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use log::info;
use thrift::protocol::{
    TBinaryInputProtocol, TBinaryOutputProtocol, TCompactInputProtocol,
    TCompactInputProtocolFactory, TCompactOutputProtocol,
};
use thrift::transport::{
    ReadHalf, TFramedReadTransport, TFramedWriteTransport, TIoChannel, TTcpChannel, WriteHalf,
};
use thrift::TThriftClient;

use crate::common::error;
use crate::datamodel::GlobalDataArc;
use crate::event_io_processor::thrift::rufsm;
use crate::event_io_processor::thrift::rufsm::{
    EventProcessorSyncClient, EventProcessorSyncHandler, EventProcessorSyncProcessor,
    TEventProcessorSyncClient,
};
use crate::event_io_processor::{EventIOProcessor, ExternalQueueContainer};
use crate::fsm::{Event, SessionId};
use crate::fsm_executor::ExecutorStateArc;

pub const THRIFT_EVENT_PROCESSOR: &str = "http://www.w3.org/TR/scxml/#ThriftEventProcessor";

/// W3C: Processors MAY define short form notations as an authoring convenience
pub const THRIFT_EVENT_PROCESSOR_SHORT_TYPE: &str = "thrift";

pub struct MyServer {
    client: EventProcessorSyncClient<
        TCompactInputProtocol<TFramedReadTransport<ReadHalf<TTcpChannel>>>,
        TCompactOutputProtocol<TFramedWriteTransport<WriteHalf<TTcpChannel>>>,
    >,
}

impl Debug for MyServer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

/// IO Processor
#[derive(Debug, Clone)]
pub struct ThriftEventIOProcessor {
    pub executor_state: ExecutorStateArc,
    pub sessions: ExternalQueueContainer,
    pub client: Option<Arc<Mutex<Box<MyServer>>>>,
    pub sender: Sender<ThriftEvent>,
}

pub struct ThriftEvent {
    pub event: Event,
    pub target: String,
}

const TYPES: &[&str] = &[THRIFT_EVENT_PROCESSOR, THRIFT_EVENT_PROCESSOR_SHORT_TYPE];

impl ThriftEventIOProcessor {
    pub fn new(execute_state: ExecutorStateArc) -> ThriftEventIOProcessor {
        println!("connect to server on 127.0.0.1:50000");

        let mut c = TTcpChannel::new();

        println!("TTcpChannel open");
        let cr = c.open("127.0.0.1:50000");

        println!(" -> {:?}", cr);

        let (sender, receiver) = channel();

        println!("New Processor:");

        let mut processor = ThriftEventIOProcessor {
            executor_state: execute_state.clone(),
            sessions: Default::default(),
            client: None,
            sender,
        };
        println!("ok");

        println!("split tcp channel");

        match c.split() {
            Ok((i_chan, o_chan)) => {
                println!("protocols");
                let mut i_prot = TBinaryInputProtocol::new(i_chan, false);
                let mut o_prot = TBinaryOutputProtocol::new(o_chan, false);

                /*
                let i_prot = TCompactInputProtocol::new( TFramedReadTransport::new(i_chan) );
                let o_prot = TCompactOutputProtocol::new( TFramedWriteTransport::new(o_chan) );
                 */

                println!("create client");
                let mut client = EventProcessorSyncClient::new(i_prot, o_prot);

                println!("register_fsm");
                let t = client.register_fsm("localhost:50001".to_string());
                println!(" -> {:?}", t);

                /*
                let _ = processor.client.insert(Arc::new(Mutex::new(Box::new(MyServer {
                    client: client,
                }))));

                // =========== Server ===========
                let i_tran_fact = TFramedReadTransportFactory::new();
                let i_prot_fact = TCompactInputProtocolFactory::new();

                let o_tran_fact = TFramedWriteTransportFactory::new();
                let o_prot_fact = TCompactOutputProtocolFactory::new();

                // demux incoming messages
                let thrift_processor = RufsmSyncProcessor::new(RufsmSyncServer {
                });

                // create the server and start listening
                let mut server = TServer::new(
                    i_tran_fact,
                    i_prot_fact,
                    o_tran_fact,
                    o_prot_fact,
                    thrift_processor,
                    5,
                );

                 info!("listen to 127.0.0.1:9090");
                 server.listen(&"127.0.0.1:9090");
                 info!("Listen finished");
                  */
            }
            Err(e) => {
                println!("Failed {}", e)
            }
        }

        processor
    }
}

struct RufsmSyncServer {}

// Thrift handler
impl EventProcessorSyncHandler for RufsmSyncServer {
    fn handle_register_fsm(&self, client_address: String) -> thrift::Result<String> {
        info!("received: register_fsm");
        Ok("Hello".to_string())
    }

    fn handle_send_event(&self, fsm_id: String, event: rufsm::Event) -> thrift::Result<()> {
        Ok(())
    }
}

impl EventIOProcessor for ThriftEventIOProcessor {
    fn get_location(&self, id: SessionId) -> String {
        format!("{}/{}", "?", id)
    }

    fn get_types(&self) -> &[&str] {
        TYPES
    }

    fn get_external_queues(&mut self) -> &mut ExternalQueueContainer {
        &mut self.sessions
    }

    fn get_copy(&self) -> Box<dyn EventIOProcessor> {
        let b = ThriftEventIOProcessor {
            executor_state: self.executor_state.clone(),
            sessions: self.sessions.clone(),
            client: self.client.clone(),
            sender: self.sender.clone(),
        };
        Box::new(b)
    }

    fn send(&mut self, global: &GlobalDataArc, target: &str, event: Event) -> bool {
        let r = self.sender.send(ThriftEvent {
            event: event,
            target: target.to_string(),
        });
        match r {
            Ok(()) => true,
            Err(_) => false,
        }
    }

    fn shutdown(&mut self) {
        let _ = self.sender.send(ThriftEvent {
            event: Default::default(),
            target: "SHUTDOWN".to_string(),
        });
    }
}
