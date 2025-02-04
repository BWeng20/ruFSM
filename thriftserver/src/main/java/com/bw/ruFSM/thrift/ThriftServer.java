package com.bw.ruFSM.thrift;

import org.apache.thrift.server.TServer;
import org.apache.thrift.server.TSimpleServer;
import org.apache.thrift.transport.TServerSocket;
import org.apache.thrift.transport.TServerTransport;

import java.net.InetSocketAddress;

public class ThriftServer {

    static final long start = System.currentTimeMillis();

    static void log(String message) {
        long t = System.currentTimeMillis()-start;
        System.out.printf("[%3d.%02ds] %s\n", (t / 1000), t % 1000, message );
    }




    RuFsmHandler handler;
    rufsm.Processor processor;

    TServer server;

    public ThriftServer() {
        handler = new RuFsmHandler();
        processor = new rufsm.Processor<>(handler);
        Runnable simple = new Runnable() {
            public void run() {
                try {
                    TServerTransport serverTransport = new TServerSocket( new InetSocketAddress("127.0.0.1", 50000));
                    server = new TSimpleServer(new TServer.Args(serverTransport).processor(processor));
                    log("Starting the simple server...");
                    server.serve();
                    log("Server stopped");
                } catch (Exception e) {
                    e.printStackTrace();
                }
            }
        };
        new Thread(simple).start();
        /*
        try {
            Thread.sleep(5000);
        } catch (InterruptedException e) {
        }
        log("Shutting down...");
        server.stop();
        */
    }

    public static void main(String[] args) {
        ThriftServer server = new ThriftServer();
    }

}
