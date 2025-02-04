//! I/O Processor implementation using MQTT

use rumqttc::v5::MqttOptions;

use crate::datamodel::GlobalDataArc;
use crate::event_io_processor::{EventIOProcessor, ExternalQueueContainer};
use crate::fsm::{Event, SessionId};
use crate::fsm_executor::ExecutorStateArc;

pub const MQTT_EVENT_PROCESSOR: &str = "http://www.w3.org/TR/scxml/#MqttEventProcessor";

/// W3C: Processors MAY define short form notations as an authoring convenience
pub const MQTT_EVENT_PROCESSOR_SHORT_TYPE: &str = "mqtt";


/// IO Processor
#[derive(Debug, Clone)]
pub struct MqttEventIOProcessor {
    pub executor_state: ExecutorStateArc,
    pub options: MqttOptions,
}


const TYPES: &[&str] = &[MQTT_EVENT_PROCESSOR, MQTT_EVENT_PROCESSOR_SHORT_TYPE];


impl MqttEventIOProcessor {
    pub fn new( execute_state: ExecutorStateArc, options: MqttOptions ) -> MqttEventIOProcessor {
        let es_clone = execute_state.clone();

        MqttEventIOProcessor {
            executor_state : es_clone,
            options
        }
    }
}

impl EventIOProcessor for MqttEventIOProcessor {
    fn get_location(&self, id: SessionId) -> String {
        format!("{}/{}", self.options.client_id(), id)
    }

    fn get_types(&self) -> &[&str] {
        TYPES
    }

    fn get_external_queues(&mut self) -> &mut ExternalQueueContainer {
        todo!()
    }

    fn get_copy(&self) -> Box<dyn EventIOProcessor> {
        let b = MqttEventIOProcessor {
            executor_state: self.executor_state.clone(),
            options: self.options.clone(),
        };
        Box::new(b)
    }

    fn send(&mut self, global: &GlobalDataArc, target: &str, event: Event) -> bool {
        todo!()
    }

    fn shutdown(&mut self) {
        todo!()
    }
}

