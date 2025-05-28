use std::{any::TypeId};

pub trait Component: 'static + Clone {
    fn get_type_id(&self) -> TypeId;
}



#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockComponent;
    
    impl Component for MockComponent {
        fn get_type_id(&self) -> TypeId {
            TypeId::of::<MockComponent>()
        }
    }
    
    #[test]
    fn test_component() {
        let component = MockComponent;
        let cloned_component = component.clone();
        assert_eq!(component.get_type_id(), cloned_component.get_type_id());
    }
}