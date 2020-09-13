pub struct Signal<T: Clone> {
    subs: Vec<Subscription<T>>,
    new_id: SubscriptionId,
}

struct Subscription<T> {
    id: SubscriptionId,
    callback: Box<dyn Fn(T)>,
}

pub type SubscriptionId = usize;

impl<T: Clone> Signal<T> {
    pub fn new() -> Signal<T> {
        Signal {
            subs: Vec::with_capacity(0),
            new_id: 0,
        }
    }

    pub fn connect<F>(&mut self, callback: F) -> SubscriptionId
        where F: (Fn(T)) + 'static
    {
        let id = self.new_id;
        self.new_id += 1;

        self.subs.push(Subscription {
            id: id,
            callback: Box::new(callback),
        });
        self.subs.shrink_to_fit();

        id
    }

    pub fn raise(&self, value: T) {
        for sub in self.subs.iter() {
            (sub.callback)(value.clone())
        }
    }

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
