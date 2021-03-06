/// Implements a broadcast-listener / callback / observable pattern.
///
/// `Signal` holds a list of subscriptions, each with a callback closure to run
/// on the next broadcast.
///
/// As `rt-graph` uses GTK, the terminology (`Signal` struct and its method names) match
/// GTK's terms.
pub struct Signal<T: Clone> {
    subs: Vec<Subscription<T>>,
    new_id: usize,
}

struct Subscription<T> {
    id: SubscriptionId,
    callback: Box<dyn Fn(T)>,
}

/// The identifier for a subscription, used to disconnect it when no longer required.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SubscriptionId(usize);

impl<T: Clone> Signal<T> {
    /// Construct a new `Signal`.
    pub fn new() -> Signal<T> {
        Signal {
            subs: Vec::with_capacity(0),
            new_id: 0,
        }
    }

    /// Connect a new subscriber that will receive callbacks when the
    /// signal is raised.
    ///
    /// Returns a SubscriptionId to disconnect the subscription when
    /// no longer required.
    pub fn connect<F>(&mut self, callback: F) -> SubscriptionId
        where F: (Fn(T)) + 'static
    {
        let id = SubscriptionId(self.new_id);
        self.new_id = self.new_id.checked_add(1).expect("No overflow");

        self.subs.push(Subscription {
            id,
            callback: Box::new(callback),
        });
        self.subs.shrink_to_fit();

        id
    }

    /// Notify existing subscribers.
    pub fn raise(&self, value: T) {
        for sub in self.subs.iter() {
            (sub.callback)(value.clone())
        }
    }

    /// Disconnect an existing subscription.
    pub fn disconnect(&mut self, id: SubscriptionId) {
        self.subs.retain(|sub| sub.id != id);
        self.subs.shrink_to_fit();
    }
}

#[cfg(test)]
mod test {
    use crate::Signal;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn signal() {
        let mut sig = Signal::new();

        let data: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        assert_eq!(data.get(), 0);

        let dc = data.clone();
        let subid = sig.connect(move |v| {
            dc.set(dc.get() + v);
        });
        assert_eq!(data.get(), 0);

        sig.raise(1);
        assert_eq!(data.get(), 1);

        sig.raise(2);
        assert_eq!(data.get(), 3);

        sig.disconnect(subid);

        sig.raise(0);
        assert_eq!(data.get(), 3);
    }

    #[test]
    fn signal_multiple_subscriptions() {
        let mut sig = Signal::new();

        let data: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        assert_eq!(data.get(), 0);

        let dc = data.clone();
        let sub1 = sig.connect(move |_v| {
            dc.set(dc.get() + 1);
        });
        let dc = data.clone();
        let sub2 = sig.connect(move |_v| {
            dc.set(dc.get() + 10);
        });

        sig.raise(0);
        assert_eq!(data.get(), 11);

        sig.disconnect(sub1);

        sig.raise(0);
        assert_eq!(data.get(), 21);

        sig.disconnect(sub2);
        sig.raise(0);

        sig.raise(0);
        assert_eq!(data.get(), 21);
    }
}
