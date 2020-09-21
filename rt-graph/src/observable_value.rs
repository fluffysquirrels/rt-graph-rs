/// A value that implements the Observer pattern.
///
/// Consumers can connect to receive callbacks when the value changes.
pub struct ObservableValue<T>
    where T: Clone
{
    value: T,
    subs: Vec<Subscription<T>>,
    new_id: usize,
}

struct Subscription<T> {
    id: SubscriptionId,
    callback: Box<dyn Fn(&T)>
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SubscriptionId(usize);

impl<T> ObservableValue<T>
    where T: Clone
{
    pub fn new(initial_value: T) -> ObservableValue<T> {
        ObservableValue {
            value: initial_value,
            new_id: 0,
            subs: Vec::with_capacity(0),
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn set(&mut self, new_value: &T) {
        self.value = new_value.clone();
        self.call_subscribers();
    }

    fn call_subscribers(&self) {
        for sub in self.subs.iter() {
            (sub.callback)(&self.value)
        }
    }

    pub fn connect<F>(&mut self, callback: F) -> SubscriptionId
        where F: (Fn(&T)) + 'static
    {
        let id = SubscriptionId(self.new_id);
        self.new_id += 1;
        self.subs.push(Subscription {
            id,
            callback: Box::new(callback),
        });
        self.subs.shrink_to_fit();
        id
    }

    pub fn disconnect(&mut self, sub_id: SubscriptionId) {
        self.subs.retain(|sub| sub.id != sub_id);
        self.subs.shrink_to_fit();
    }
}

#[cfg(test)]
mod test {
    use std::{
        cell::Cell,
        rc::Rc,
    };
    use super::ObservableValue;

    #[test]
    fn new_get_set() {
        let mut ov = ObservableValue::new(17);
        assert_eq!(*ov.get(), 17);

        ov.set(&18);
        assert_eq!(*ov.get(), 18);
    }

    #[test]
    fn connect_set() {
        let mut ov = ObservableValue::<u32>::new(17);
        let mirror: Rc<Cell<u32>> = Rc::new(Cell::new(0));

        let mc = mirror.clone();
        ov.connect(move |val| {
            mc.set(*val);
        });

        // Check callback not yet called.
        assert_eq!(mirror.get(), 0);

        ov.set(&18);

        // Check the callback was called with the correct value.
        assert_eq!(mirror.get(), 18);
    }

    #[test]
    fn disconnect() {
        let mut ov = ObservableValue::<u32>::new(17);
        let mirror_1: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let mirror_2: Rc<Cell<u32>> = Rc::new(Cell::new(0));

        let mc1 = mirror_1.clone();
        let sub_id_1 = ov.connect(move |val| {
            mc1.set(*val);
        });

        let mc2 = mirror_2.clone();
        let _sub_id_2 = ov.connect(move |val| {
            mc2.set(*val);
        });

        // Both mirrors are connected with callbacks, set() updates both mirror values.
        ov.set(&18);
        assert_eq!(mirror_1.get(), 18);
        assert_eq!(mirror_2.get(), 18);

        ov.disconnect(sub_id_1);

        // Only sub_id_2 is still connected, set() only updates one mirror value.
        ov.set(&19);
        assert_eq!(mirror_1.get(), 18);
        assert_eq!(mirror_2.get(), 19);
    }
}
