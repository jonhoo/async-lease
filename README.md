# async-lease

[![Crates.io](https://img.shields.io/crates/v/async-lease.svg)](https://crates.io/crates/async-lease)
[![Documentation](https://docs.rs/async-lease/badge.svg)](https://docs.rs/async-lease/)
[![Build Status](https://travis-ci.com/jonhoo/async-lease.svg?branch=master)](https://travis-ci.com/jonhoo/async-lease)
[![Codecov](https://codecov.io/github/jonhoo/async-lease/coverage.svg?branch=master)](https://codecov.io/gh/jonhoo/async-lease)

An asynchronous, atomic option type intended for use with methods that move `self`.

This module provides `Lease`, a type that acts similarly to an asynchronous `Mutex`, with one
major difference: it expects you to move the leased item _by value_, and then _return it_ when
you are done. You can think of a `Lease` as an atomic, asynchronous `Option` type, in which we
can `take` the value only if no-one else has currently taken it, and where we are notified when
the value has returned so we can try to take it again.

This type is intended for use with methods that take `self` by value, and _eventually_, at some
later point in time, return that `Self` for future use. This tends to happen particularly often
in future-related contexts. For example, consider the following method for a hypothetical,
non-pipelined connection type:

```rust
impl Connection {
    fn get(self, key: i64) -> impl Future<Item = (i64, Self), Error = Error>;
}
```

Let's say you want to expose an interface that does _not_ consume `self`, but instead has a
`poll_ready` method that checks whether the connection is ready to receive another request:

```rust
impl MyConnection {
    fn poll_ready(&mut self) -> Poll<(), Error = Error>;
    fn call(&mut self, key: i64) -> impl Future<Item = i64, Error = Error>;
}
```

`Lease` allows you to do this. Specifically, `poll_ready` attempts to acquire the lease using
`Lease::poll_acquire`, and `call` _transfers_ that lease into the returned future. When the
future eventually resolves, we _restore_ the leased value so that `poll_ready` returns `Ready`
again to anyone else who may want to take the value. The example above would thus look like
this:

```rust
impl MyConnection {
    fn poll_ready(&mut self) -> Poll<(), Error = Error> {
        self.lease.poll_acquire()
    }

    fn call(&mut self, key: i64) -> impl Future<Item = i64, Error = Error> {
        // We want to transfer the lease into the future
        // and leave behind an unacquired lease.
        let mut lease = self.lease.transfer();
        lease.take().get(key).map(move |(v, connection)| {
            // Give back the connection for other callers.
            // After this, `poll_ready` may return `Ok(Ready)` again.
            lease.restore(connection);
            // And yield just the value.
            v
        })
    }
}
```
