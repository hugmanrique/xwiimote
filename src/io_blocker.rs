use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::thread;

use crate::{bail_if, Result};

/// Listens for events from all monitors and devices associated
/// with the application.
pub(crate) struct IoBlocker {
    ep_fd: RawFd,
    wakers: Mutex<HashMap<RawFd, Waker>>,
}

impl IoBlocker {
    pub fn get() -> &'static Self {
        static BLOCKER: Lazy<IoBlocker> = Lazy::new(|| {
            thread::spawn(move || {
                let blocker = IoBlocker::get();
                blocker.run().expect("event loop failed");
            });

            // Create epoll instance
            let ep_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
            assert_ne!(ep_fd, -1, "failed to create epoll instance");

            IoBlocker {
                ep_fd,
                wakers: Mutex::new(HashMap::new()),
            }
        });
        &BLOCKER
    }

    /// Executes the event loop.
    fn run(&self) -> Result<()> {
        let term = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;

        // Reuse the readiness events vector across `wake_ready` calls.
        let mut events = Vec::with_capacity(16);
        while !term.load(Ordering::Relaxed) {
            self.wake_ready(&mut events)?;
        }

        unsafe { libc::close(self.ep_fd) };
        Ok(())
    }

    /// Blocks until one or more events occurs, and wakes the futures that
    /// expressed interest in them.
    fn wake_ready(&self, events: &mut Vec<libc::epoll_event>) -> Result<()> {
        events.clear();
        let n_ready = unsafe {
            libc::epoll_wait(
                self.ep_fd,
                events.as_mut_ptr(),
                events.capacity() as libc::c_int,
                -1, // todo: set reasonable timeout
            )
        };
        bail_if!(n_ready == -1);

        // Safety: `epoll_wait` ensures `n_ready` events are assigned.
        unsafe { events.set_len(n_ready as usize) };

        let mut wakers = self.wakers.lock().unwrap();
        for event in events.iter() {
            let fd = event.u64 as RawFd;
            if let Some(waker) = wakers.remove(&fd) {
                waker.wake();
            }
        }
        Ok(())
    }

    fn ctl_interest(&self, op: libc::c_int, fd: RawFd, events: libc::c_int) -> Result<()> {
        let mut event = libc::epoll_event {
            // Enable edge-triggered mechanism, caller is expected to
            // read all available data from `fd`.
            events: (events | libc::EPOLLET) as libc::c_uint,
            u64: fd.try_into().unwrap(),
        };
        let res_code = unsafe { libc::epoll_ctl(self.ep_fd, op, fd, &mut event) };
        bail_if!(res_code == -1);
        Ok(())
    }

    /// Expresses an interest in a particular event on the file.
    pub fn add_interest(&self, fd: RawFd, events: libc::c_int) -> Result<()> {
        self.ctl_interest(libc::EPOLL_CTL_ADD, fd, events)
    }

    /// Removes the interest in a particular event on the file.
    ///
    /// This also wakes the pending future, if set.
    pub fn remove_interest(&self, fd: RawFd, events: libc::c_int) -> Result<()> {
        self.ctl_interest(libc::EPOLL_CTL_DEL, fd, events)?;
        if let Some(waker) = self.wakers.lock().unwrap().remove(&fd) {
            waker.wake();
        }
        Ok(())
    }

    /// Stores the waker to be called once an IO event on the file
    /// arrives.
    ///
    /// The future is expected to read all available data from `fd`
    /// once waken up. Otherwise the event loop can block indefinitely.
    pub fn set_callback(&self, fd: RawFd, waker: Waker) {
        self.wakers.lock().unwrap().insert(fd, waker);
    }
}

#[cfg(test)]
mod tests {
    use crate::{IoBlocker, Result};
    use futures::executor;
    use std::future::Future;

    use std::pin::Pin;

    use std::task::{Context, Poll};

    #[test]
    fn double_interest_fails() {
        let blocker = IoBlocker::get();
        blocker.add_interest(0, libc::EPOLLIN).unwrap();
        assert!(blocker.add_interest(0, libc::EPOLLIN).is_err());
    }

    // todo: the following assumes that stdout is open.

    #[test]
    fn event_wakes_future() -> Result<()> {
        const FD: libc::c_int = libc::STDOUT_FILENO;
        IoBlocker::get().add_interest(FD, libc::EPOLLOUT)?;

        struct StdoutFuture(bool);
        impl Future for StdoutFuture {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.0 {
                    // First try, let `IoBlocker` wake us up for the second try.
                    IoBlocker::get().set_callback(FD, cx.waker().clone());
                    self.0 = false;
                    Poll::Pending
                } else {
                    // Second try, we're done.
                    Poll::Ready(())
                }
            }
        }

        executor::block_on(StdoutFuture(true));
        Ok(())
    }
}
