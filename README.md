# chatserver_rust
Rewrite of a simple chatserver into rust.

Just messing around with rust and decided to rewrite [a rudimentary multithreaded chatserver](https://github.com/dmsalomon/chatserver) into rust. I have only done the server, since the C client is compatible with the rust server (and the protocol is so simple that you can use a raw sockets client, i.e. nc or telnet).
