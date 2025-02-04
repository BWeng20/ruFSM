package com.bw.ruFSM.thrift;

import org.apache.thrift.TException;

public class RuFsmHandler implements rufsm.Iface {

    int idCount = 0;

    @Override
    public String registerFsm() throws TException {
        System.out.println("CALLED registerFsm");
        return "fsm"+(++idCount);
    }
}
