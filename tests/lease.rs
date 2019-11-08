use async_lease::Lease;
use tokio_test::*;

#[test]
fn default() {
    task::mock(|cx| {
        let mut l: Lease<bool> = Lease::default();
        assert_ready!(l.poll_acquire(cx));
        assert_eq!(&*l, &false);
        assert_eq!(l.take(), false);
        l.restore(true);
    });
}

#[test]
fn straight_execution() {
    task::mock(|cx| {
        let mut l = Lease::from(100);

        // We can immediately acquire the lease and take the value
        assert_ready!(l.poll_acquire(cx));
        assert_eq!(&*l, &100);
        assert_eq!(l.take(), 100);
        l.restore(99);

        // We can immediately acquire again since the value was returned
        assert_ready!(l.poll_acquire(cx));
        assert_eq!(l.take(), 99);
        l.restore(98);

        // Dropping the lease is okay since we returned the value
        drop(l);
    });
}

#[test]
fn drop_while_acquired_ok() {
    task::mock(|cx| {
        let mut l = Lease::from(100);
        assert_ready!(l.poll_acquire(cx));

        // Dropping the lease while it is still acquired shouldn't
        // be an issue since we haven't taken the leased value.
        drop(l);
    });
}

#[test]
#[should_panic]
fn take_twice() {
    task::mock(|cx| {
        let mut l = Lease::from(100);

        assert_ready!(l.poll_acquire(cx));
        assert_eq!(l.take(), 100);
        l.take(); // should panic
    });
}

#[test]
#[should_panic]
fn mut_after_take() {
    task::mock(|cx| {
        let mut l = Lease::from(100);

        assert_ready!(l.poll_acquire(cx));
        // at this point we have the lease, so we can mutate directly
        *l = 99;
        // then we can take
        assert_eq!(l.take(), 99);
        // but now we shouldn't be allowed to mutate any more!
        *l = 98;
    });
}

#[test]
#[should_panic]
fn take_wo_acquire() {
    let mut l = Lease::from(100);
    l.take(); // should panic
}

#[test]
#[should_panic]
fn drop_without_restore() {
    task::mock(|cx| {
        let mut l = Lease::from(100);
        assert_ready!(l.poll_acquire(cx));
        assert_eq!(l.take(), 100);
        drop(l); // should panic
    });
}

#[test]
#[should_panic]
fn release_after_take() {
    task::mock(|cx| {
        let mut l = Lease::from(100);
        assert_ready!(l.poll_acquire(cx));
        assert_eq!(l.take(), 100);
        l.release(); // should panic
    });
}

#[test]
fn transfer_lease() {
    task::mock(|cx| {
        let mut l = Lease::from(100);

        assert_ready!(l.poll_acquire(cx));

        // We should be able to transfer the acquired lease
        let mut l2 = l.transfer();
        // And then use it as normal
        assert_eq!(&*l2, &100);
        assert_eq!(l2.take(), 100);
        l2.restore(99);

        // Dropping the transferred lease is okay since we returned the value
        drop(l2);

        // Once the transferred lease has been restored, we can acquire the lease again
        assert_ready!(l.poll_acquire(cx));
        assert_eq!(l.take(), 99);
        l.restore(98);
    });
}

#[test]
fn readiness() {
    let mut task = task::MockTask::new();

    let mut l = Lease::from(100);
    task.enter(|cx| {
        assert_ready!(l.poll_acquire(cx));
    });
    let mut l2 = l.transfer();

    // We can't now acquire the lease since it's already held in l2
    task.enter(|cx| {
        assert_pending!(l.poll_acquire(cx));
    });

    // But once l2 restores the value, we can acquire it
    l2.restore(99);
    assert!(task.is_woken());
    task.enter(|cx| {
        assert_ready!(l.poll_acquire(cx));
    });
}
