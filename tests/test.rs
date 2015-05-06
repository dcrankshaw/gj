// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

extern crate gj;

#[test]
fn eval_void() {
    use std::rc::Rc;
    use std::cell::Cell;
    gj::EventLoop::init();
    let done = Rc::new(Cell::new(false));
    let done1 = done.clone();
    let promise = gj::Promise::fulfilled(()).map(move |()| {
        done1.clone().set(true);
        return Ok(());
    });
    assert_eq!(done.get(), false);
    promise.wait().unwrap();
    assert_eq!(done.get(), true);
}

#[test]
fn eval_int() {
    gj::EventLoop::init();
    let promise = gj::Promise::fulfilled(19u64).map(|x| {
        assert_eq!(x, 19);
        return Ok(x + 2);
    });
    let value = promise.wait().unwrap();
    assert_eq!(value, 21);
}


#[test]
fn fulfiller() {
    gj::EventLoop::init();
    let (promise, mut fulfiller) = gj::new_promise_and_fulfiller::<u32>();
    let p1 = promise.map(|x| {
        assert_eq!(x, 10);
        return Ok(x + 1);
    });

    fulfiller.fulfill(10);
    let value = p1.wait().unwrap();
    assert_eq!(value, 11);

}

#[test]
fn chain() {
    gj::EventLoop::init();

    let promise: gj::Promise<i32> = gj::Promise::fulfilled(()).map(|()| { return Ok(123); });
    let promise2: gj::Promise<i32> = gj::Promise::fulfilled(()).map(|()| { return Ok(321); });

    let promise3 = promise.then(move |i| {
        return Ok(promise2.then(move |j| {
            return Ok(gj::Promise::fulfilled(i + j));
        }));
    });

    let value = promise3.wait().unwrap();
    assert_eq!(444, value);
}

#[test]
fn chain_error() {
    gj::EventLoop::init();

    let promise = gj::Promise::fulfilled(()).map(|()| { return Ok("123"); });
    let promise2 = gj::Promise::fulfilled(()).map(|()| { return Ok("XXX321"); });

    let promise3 = promise.then(move |istr| {
        return Ok(promise2.then(move |jstr| {
            let i: i32 = try!(istr.parse());
            let j: i32 = try!(jstr.parse());  // Should return an error.
            return Ok(gj::Promise::fulfilled(i + j));
        }));
    });

    assert!(promise3.wait().is_err());
}

#[test]
fn deep_chain2() {
    gj::EventLoop::init();

    let mut promise = gj::Promise::fulfilled(4u32);

    for _ in 0..1000 {
        promise = gj::Promise::fulfilled(()).then(|_| {
            return Ok(promise);
        });
    }

    let value = promise.wait().unwrap();

    assert_eq!(value, 4);
}

#[test]
fn ordering() {
    use std::rc::Rc;
    use std::cell::{Cell, RefCell};

    gj::EventLoop::init();

    let counter = Rc::new(Cell::new(0u32));
    let counter0 = counter.clone();
    let mut promises: Vec<Rc<RefCell<Option<gj::Promise<()>>>>> = Vec::new();
    for _ in 0..6 {
        promises.push(Rc::new(RefCell::new(None)));
    }

    let promise2 = promises[2].clone();
    let promise3 = promises[3].clone();
    *promises[0].borrow_mut() = Some(gj::Promise::fulfilled(()).then(move |_| {
        assert_eq!(counter0.get(), 0);
        counter0.set(1);

        {
            // Use a promise and fulfiller so that we can fulfill the promise after waiting on it in
            // order to induce depth-first scheduling.
            let (promise, fulfiller) = gj::new_promise_and_fulfiller::<()>();
            let counter1 = counter0.clone();
            *promise2.borrow_mut() = Some(promise.map(move |_| {
                assert_eq!(counter1.get(), 1);
                counter1.set(2);
                return Ok(());
            }));
            fulfiller.fulfill(());
        }

        let counter4 = counter.clone();
        // .map() is scheduled breadth-first is the promise has already resolved, but depth-first
        // if the promise resolves later.
        *promise3.borrow_mut() = Some(gj::Promise::fulfilled(()).map(move |_| {
            assert_eq!(counter4.get(), 2);
            return Ok(());
        }));

        return Ok(gj::Promise::fulfilled(()));
    }));

    for p in promises.into_iter() {
        let maybe_p = ::std::mem::replace(&mut *p.borrow_mut(), None);
        match maybe_p {
            None => {}
            Some(p) => {
                p.wait().unwrap()
            }
        }
    }
}
