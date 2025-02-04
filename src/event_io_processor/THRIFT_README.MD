# Thrift I/O Processor code generation

If the thrift Idl files have changed, the generated thrift stubs must re-generated,

First get the thrift compiler from https://thrift.apache.org/download.
Currently, we use thrift-0.21.0. Unpack the archive somewhere.

Open a bash inside the folder (in windows use wsl). Then follow the instructions from the thrift webpage.

```
sudo apt-get update
sudo apt-get install automake bison flex g++ git libboost-all-dev libevent-dev libssl-dev libtool make pkg-config

# In my wsl the build failed without this: 
export MAKE="gmake"

./bootstrap.sh
./configure
make
sudo make install
```

> [!NOTE]
> The tests in 0.21.0 seems to fail with some error message (in make and make install).
> Ignore it!


I guess it is important (at least for this project), that thrift locates rust, so check the output from `configure`.
It should contain:

```
thrift 0.21.0
...
Building Rust Library ........ : yes
```

After thrift was installed, generate the rust sources for the interface files.
Go to the project directory root directory and execute:

```
thrift -v --gen rs -out src/event_io_processor/thrift thriftserver/rufsm.thrift
```
