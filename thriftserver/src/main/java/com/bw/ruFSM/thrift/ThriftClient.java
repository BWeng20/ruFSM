package com.bw.ruFSM.thrift;

import org.apache.thrift.protocol.TBinaryProtocol;
import org.apache.thrift.protocol.TProtocol;
import org.apache.thrift.server.TServer;
import org.apache.thrift.server.TSimpleServer;
import org.apache.thrift.transport.TServerSocket;
import org.apache.thrift.transport.TServerTransport;
import org.apache.thrift.transport.TSocket;
import org.apache.thrift.transport.TTransport;

import java.net.InetSocketAddress;

public class ThriftClient {

    static final long start = System.currentTimeMillis();

    static void log(String message) {
        long t = System.currentTimeMillis()-start;
        System.out.printf("[%3d.%02ds] %s\n", (t / 1000), t % 1000, message );
    }

    ThriftClient client;

    public ThriftClient() {
        try {
            TTransport transport = new TSocket("127.0.0.1", 50000);
            transport.open();

            TProtocol protocol = new TBinaryProtocol(transport);
            rufsm.Client client = new rufsm.Client(protocol);

            String c = client.registerFsm();
            System.out.println("Registered "+c);

            transport.close();
        } catch (Exception x) {
            x.printStackTrace();
        }
    }

    public static void main(String[] args) {
        ThriftClient server = new ThriftClient();
    }

}
