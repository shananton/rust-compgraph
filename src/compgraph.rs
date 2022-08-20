use std::{rc::{Rc, Weak}, cell::{RefCell, Cell}};

pub type Float = f32;

pub trait Compute {
    fn compute(&self) -> Float;
}

pub trait InvalidateCache {
    fn invalidate_cache(&self);
}

pub trait PublishInvalidate {
    fn subscribe_to_invalidate(&self, subscriber: &Rc<dyn InvalidateCache>);
    fn publish_invalidate(&self);
}

pub trait ComputeNode: Compute + PublishInvalidate {}

pub trait InputNode: ComputeNode {
    fn set(&self, value: Float);
}

impl Compute for Float {
    fn compute(&self) -> Float { *self }
}

impl PublishInvalidate for Float {
    // Float constants trivially satisfy this trait by never changing
    fn subscribe_to_invalidate(&self, _subscriber: &Rc<dyn InvalidateCache>) { }
    fn publish_invalidate(&self) { }
}

impl ComputeNode for Float {}

pub struct CachedNodeWrapper<T: Compute> {
    pub inner: T,
    cached_value: Cell<Option<Float>>,
    invalidate_publisher: InvalidatePublisher
}

impl<T: Compute> CachedNodeWrapper<T> {
    pub fn new(inner: T) -> CachedNodeWrapper<T> {
        CachedNodeWrapper { inner, cached_value: Cell::new(None), invalidate_publisher: InvalidatePublisher::new() }
    }
    // pub fn into_rc_invalidate_cache(self: Rc<Self>) -> Rc<dyn InvalidateCache> { self }
}

impl<T: Compute> Compute for CachedNodeWrapper<T> {
    fn compute(&self) -> Float {
        if self.cached_value.get().is_none() {
            self.cached_value.set(Some(self.inner.compute()))
        }
        self.cached_value.get().unwrap()
    }
}

impl<T: Compute> PublishInvalidate for CachedNodeWrapper<T> {
    fn subscribe_to_invalidate(&self, subscriber: &Rc<dyn InvalidateCache>) {
        self.invalidate_publisher.subscribe_to_invalidate(subscriber)
    }
    fn publish_invalidate(&self) {
        self.invalidate_publisher.publish_invalidate()
    }
}

impl<T: Compute> ComputeNode for CachedNodeWrapper<T> {}

impl<T: Compute> InvalidateCache for CachedNodeWrapper<T> {
    fn invalidate_cache(&self) {
        self.cached_value.set(None);
        self.invalidate_publisher.publish_invalidate();
    }
}

pub struct InvalidatePublisher {
    subscribers: RefCell<Vec<Weak<dyn InvalidateCache>>>
}

impl InvalidatePublisher {
    pub fn new() -> InvalidatePublisher {
        InvalidatePublisher { subscribers: RefCell::new(Vec::new()) }
    }
}

impl PublishInvalidate for InvalidatePublisher {
    fn subscribe_to_invalidate(&self, subscriber: &Rc<dyn InvalidateCache>) {
        self.subscribers.borrow_mut().push(Rc::downgrade(subscriber))
    }
    fn publish_invalidate(&self) {
        self.subscribers.borrow_mut().retain(|dep_weak| {
            dep_weak.upgrade().map_or(false, |dep_rc| {
                dep_rc.invalidate_cache();
                true
            })
        })
    }
}

struct InputNodeImpl {
    value: Cell<Float>,
    invalidate_publisher: InvalidatePublisher
}

impl Compute for InputNodeImpl {
    fn compute(&self) -> Float {
        self.value.get()
    }
}

impl PublishInvalidate for InputNodeImpl {
    fn subscribe_to_invalidate(&self, subscriber: &Rc<dyn InvalidateCache>) {
        self.invalidate_publisher.subscribe_to_invalidate(subscriber)
    }    
    fn publish_invalidate(&self) {
        self.invalidate_publisher.publish_invalidate()
    }
}

impl ComputeNode for InputNodeImpl {}

impl InputNode for InputNodeImpl {
    fn set(&self, value: Float) {
        self.value.set(value)
    }
}

pub type ComputeNodeRef = Rc<dyn ComputeNode>;

impl Compute for ComputeNodeRef {
    fn compute(&self) -> Float {
        Rc::as_ref(self).compute()
    }
}

impl PublishInvalidate for ComputeNodeRef {
    fn subscribe_to_invalidate(&self, subscriber: &Rc<dyn InvalidateCache>) {
        Rc::as_ref(self).subscribe_to_invalidate(subscriber)
    }
    fn publish_invalidate(&self) {
        Rc::as_ref(self).publish_invalidate()
    }
}

impl ComputeNode for ComputeNodeRef {}

#[macro_export]
macro_rules! define_node {
    ($struct_name:ident computes $fun_name:ident($($params:ident),+) $body:block) => {
        #[allow(non_camel_case_types)]
        struct $struct_name<$($params: $crate::compgraph::ComputeNode),+> {
            $($params: $params),+
        }

        #[allow(non_camel_case_types)]
        impl<$($params: $crate::compgraph::ComputeNode),+> Compute for $struct_name<$($params),+> {
            fn compute(&self) -> $crate::compgraph::Float {
                $(let $params: $crate::compgraph::Float = self.$params.compute());+;
                $body
            }
        }

        pub fn $fun_name($($params: impl $crate::compgraph::ComputeNode + 'static),+) -> $crate::compgraph::ComputeNodeRef {
            let result = ::std::rc::Rc::new($crate::compgraph::CachedNodeWrapper::new(
                $struct_name { $($params),+ }
            ));
            let subscriber = result.clone() as ::std::rc::Rc<dyn $crate::compgraph::InvalidateCache>;
            $(result.inner.$params.subscribe_to_invalidate(&subscriber));+;
            result
        }
    };
}

