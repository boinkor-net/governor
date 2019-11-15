# governor - a library for regulating the flow of data

This library is an implementation of the [Generic Cell Rate Algorithm](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm)
for rate limiting in Rust programs.

It is intended to help your program know how much strain it is supposed to put on external services (and, to some extent, to
allow your services to regulate how much strain they take on from their users). In a way, it functions like the
iconic steam governor from which this library takes its name:

![a centrifugal governor](doc/centrifugal-governor.png)
