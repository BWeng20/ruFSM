namespace java com.bw.ruFSM.thrift

struct Event {
    1: string name
}

/**
 * FSM Event Processor
 */
service EventProcessor
 {
    string registerFsm(1: string clientAddress);
    oneway void send_event( 1:string fsmId, 2: Event event );
}