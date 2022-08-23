use std::{rc::{Rc, Weak}, cell::RefCell};

pub type Float = f32;

// Traits named `*Mut` are meant for the shared node structures,
// while `*Ref` are for `Rc<RefCell<Impl>>` with by-reference semantics
//
// `ComputeNodeRef` is also implemented directly for `Float`
// to allow for inlining constants without `Rc` and extra allocations

// Dependency-tracking functionality to be reused by both input and computational nodes
struct InvalidatePublisher {
    subscribers: Vec<Weak<RefCell<dyn InvalidateCacheMut>>>
}

impl InvalidatePublisher {
    pub fn new() -> InvalidatePublisher {
        InvalidatePublisher { subscribers: Vec::new() }
    }
    fn subscribe_to_invalidate(&mut self, subscriber: &Rc<RefCell<dyn InvalidateCacheMut>>) {
        self.subscribers.push(Rc::downgrade(subscriber))
    }
    fn publish_invalidate(&mut self) {
        self.subscribers.retain(|dep_weak| {
            dep_weak.upgrade().map_or(false, |dep_rc| {
                dep_rc.borrow_mut().invalidate_cache();
                true
            })
        })
    }
}

pub mod internals {
    // Things that have to be public as they are used in the expansion of `define_nodes!`
    // The name `internals` suggests that these should not be used directly

    use super::*;
    pub trait InvalidateCacheMut {
        fn invalidate_cache(&mut self);
    }

    pub trait ComputeMut {
        fn compute(&mut self) -> Float;
    }

    // Caching functionality separated out to minimize the amount of code
    // in the expansion of define_nodes!
    pub struct CachingNodeWrapper<T: ComputeMut> {
        pub inner: T,
        cached_value: Option<Float>,
        invalidate_publisher: InvalidatePublisher
    }

    impl<T: ComputeMut> CachingNodeWrapper<T> {
        pub fn new(inner: T) -> CachingNodeWrapper<T> {
            CachingNodeWrapper { inner, cached_value: None, invalidate_publisher: InvalidatePublisher::new() }
        }
    }

    impl<T: ComputeMut> ComputeMut for CachingNodeWrapper<T> {
        fn compute(&mut self) -> Float {
            let cached_value = &mut self.cached_value;
            *cached_value.get_or_insert_with(|| self.inner.compute())
        }
    }

    impl<T: ComputeMut> ComputeNodeMut for CachingNodeWrapper<T> {
        fn subscribe_to_invalidate(&mut self, subscriber: &Rc<RefCell<dyn InvalidateCacheMut>>) {
            self.invalidate_publisher.subscribe_to_invalidate(subscriber)
        }
    }

    impl<T: ComputeMut> InvalidateCacheMut for CachingNodeWrapper<T> {
        fn invalidate_cache(&mut self) {
            if self.cached_value.is_some() {
                self.cached_value = None;
                self.invalidate_publisher.publish_invalidate();
            }
        }
    }

}
use internals::*;

trait ComputeNodeMut: ComputeMut {
    fn subscribe_to_invalidate(&mut self, subscriber: &Rc<RefCell<dyn InvalidateCacheMut>>);
}


// `ComputeNodeRef` and `InputNodeRef` are the public interface traits for the user
pub trait ComputeNodeRef: Clone {
    fn compute(&self) -> Float;
    fn subscribe_to_invalidate(&self, subscriber: &Rc<RefCell<dyn InvalidateCacheMut>>);
}

pub trait InputNodeRef: ComputeNodeRef {
    fn set(&self, value: Float);
}

impl<T: ComputeNodeMut> ComputeNodeRef for Rc<RefCell<T>> {
    fn compute(&self) -> Float {
        self.borrow_mut().compute()
    }
    fn subscribe_to_invalidate(&self, subscriber: &Rc<RefCell<dyn InvalidateCacheMut>>) {
        self.borrow_mut().subscribe_to_invalidate(subscriber)
    }
}

impl ComputeNodeRef for Float {
    fn compute(&self) -> Float { *self }
    fn subscribe_to_invalidate(&self, _subscriber: &Rc<RefCell<dyn InvalidateCacheMut>>) {
        // Float constants trivially satisfy this by never changing
    }
}

struct InputNodeImpl {
    value: Float,
    invalidate_publisher: InvalidatePublisher
}

impl ComputeMut for InputNodeImpl {
    fn compute(&mut self) -> Float {
        self.value
    }
}

impl ComputeNodeMut for InputNodeImpl {
    fn subscribe_to_invalidate(&mut self, subscriber: &Rc<RefCell<dyn InvalidateCacheMut>>) {
        self.invalidate_publisher.subscribe_to_invalidate(subscriber)
    }    
}

impl InputNodeRef for Rc<RefCell<InputNodeImpl>> {
    fn set(&self, value: Float) {
        let mut inner = self.borrow_mut();
        inner.value = value;
        inner.invalidate_publisher.publish_invalidate();
    }
}

pub fn create_input() -> impl InputNodeRef {
    Rc::new(RefCell::new(InputNodeImpl { value: 0.0, invalidate_publisher: InvalidatePublisher::new() }))
}

#[macro_export]
macro_rules! define_nodes {
    {$(
        $visibility:vis $name:ident($($params:ident),+) $body:block
       )*} => {
        $(
            $visibility fn $name($($params: impl $crate::compgraph::ComputeNodeRef + 'static),+) -> impl $crate::compgraph::ComputeNodeRef {

                #[allow(non_camel_case_types)]
                struct NodeImpl<$($params: $crate::compgraph::ComputeNodeRef),+> {
                    $($params: $params),+
                }

                #[allow(non_camel_case_types)]
                impl<$($params: $crate::compgraph::ComputeNodeRef),+> $crate::compgraph::internals::ComputeMut for NodeImpl<$($params),+> {
                    fn compute(&mut self) -> $crate::compgraph::Float {
                        $(let $params: $crate::compgraph::Float = self.$params.compute());+;
                        $body
                    }
                }

                let result = ::std::rc::Rc::new(::std::cell::RefCell::new($crate::compgraph::internals::CachingNodeWrapper::new(
                    NodeImpl { $($params),+ }
                )));
                let subscriber = result.clone() as _;
                {
                    let inner = &result.borrow().inner;
                    $(inner.$params.subscribe_to_invalidate(&subscriber));+;
                }
                result
            }
        )*
    };
}

