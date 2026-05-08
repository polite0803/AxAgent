use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub type ServiceFactory = Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>;

pub struct ServiceContainer {
    factories: RwLock<HashMap<TypeId, ServiceFactory>>,
    instances: RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl ServiceContainer {
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub fn register<T: 'static + Send + Sync + ?Sized>(
        &self,
        factory: impl Fn() -> Arc<T> + Send + Sync + 'static,
    ) {
        let type_id = TypeId::of::<T>();
        let wrapped_factory: ServiceFactory =
            Box::new(move || Box::new(factory()) as Box<dyn Any + Send + Sync>);
        self.factories.write().unwrap().insert(type_id, wrapped_factory);
    }

    pub fn register_instance<T: 'static + Send + Sync + ?Sized>(&self, instance: Arc<T>) {
        let type_id = TypeId::of::<T>();
        self.instances
            .write()
            .unwrap()
            .insert(type_id, Box::new(instance));
    }

    pub fn resolve<T: 'static + Send + Sync + ?Sized>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();

        if let Some(instance) = self.instances.read().unwrap().get(&type_id) {
            return instance.downcast_ref::<Arc<T>>().cloned();
        }

        if let Some(factory) = self.factories.read().unwrap().get(&type_id) {
            let instance = factory();
            let result = instance.downcast_ref::<Arc<T>>().cloned();
            self.instances
                .write()
                .unwrap()
                .insert(type_id, instance);
            return result;
        }

        None
    }

    pub fn has<T: 'static + Send + Sync + ?Sized>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.factories.read().unwrap().contains_key(&type_id)
            || self.instances.read().unwrap().contains_key(&type_id)
    }

    pub fn reset(&self) {
        self.instances.write().unwrap().clear();
    }
}

impl Default for ServiceContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    trait Greeter: Send + Sync {
        fn greet(&self) -> String;
    }

    struct EnglishGreeter;
    impl Greeter for EnglishGreeter {
        fn greet(&self) -> String {
            "Hello!".to_string()
        }
    }

    struct CountingGreeter {
        count: AtomicU32,
    }
    impl Greeter for CountingGreeter {
        fn greet(&self) -> String {
            let n = self.count.fetch_add(1, Ordering::SeqCst);
            format!("Hello #{}!", n)
        }
    }

    #[test]
    fn test_register_and_resolve_instance() {
        let container = ServiceContainer::new();
        container.register_instance(Arc::new(EnglishGreeter) as Arc<dyn Greeter>);

        let resolved = container.resolve::<dyn Greeter>();
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().greet(), "Hello!");
    }

    #[test]
    fn test_register_factory() {
        let container = ServiceContainer::new();
        container.register::<dyn Greeter>(|| {
            Arc::new(CountingGreeter {
                count: AtomicU32::new(0),
            }) as Arc<dyn Greeter>
        });

        let first = container.resolve::<dyn Greeter>().unwrap();
        assert_eq!(first.greet(), "Hello #0!");
        assert_eq!(first.greet(), "Hello #1!");
    }

    #[test]
    fn test_resolve_missing() {
        let container = ServiceContainer::new();
        assert!(container.resolve::<dyn Greeter>().is_none());
    }

    #[test]
    fn test_has() {
        let container = ServiceContainer::new();
        assert!(!container.has::<dyn Greeter>());
        container.register_instance(Arc::new(EnglishGreeter) as Arc<dyn Greeter>);
        assert!(container.has::<dyn Greeter>());
    }

    #[test]
    fn test_reset() {
        let container = ServiceContainer::new();
        container.register_instance(Arc::new(EnglishGreeter) as Arc<dyn Greeter>);
        assert!(container.has::<dyn Greeter>());
        container.reset();
        assert!(!container.has::<dyn Greeter>());
    }
}
