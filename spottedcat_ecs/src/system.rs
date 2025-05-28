


pub trait System {
    type Item; 
    fn add_entity(&self, entity: Entity, components: Vec<Box<dyn Component>>);
    fn remove_entity(&self, entity: Entity);
    fn update(&self, delta_time: f64);
    fn destroy(&self);
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;
    use crate::components::Component;
    use std::any::TypeId;

    // Mock component for testing
    struct MockComponent;
    
    impl Component for MockComponent {
        fn get_type_id(&self) -> TypeId {
            TypeId::of::<MockComponent>()
        }
    }

    // Test implementation of System
    struct TestSystem {
        entities: Vec<Entity>,
        components: Vec<Box<dyn Component>>,
    }

    impl TestSystem {
        fn new() -> Self {
            TestSystem {
                entities: Vec::new(),
                components: Vec::new(),
            }
        }
    }

    impl System for TestSystem {
        fn add_entity(&mut self, entity: Entity, components: Vec<Box<dyn Component>>) {
            self.entities.push(entity);
            self.components.extend(components);
        }

        fn remove_entity(&mut self, entity: Entity) {
            if let Some(index) = self.entities.iter().position(|&e| e == entity) {
                self.entities.remove(index);
            }
        }

        fn update(&self, delta_time: f64) {
            // Implementation for testing
        }

        fn destroy(&self) {
            // Implementation for testing
        }
    }

    #[test]
    fn test_system_add_entity() {
        let mut system = TestSystem::new();
        let entity = Entity::new();
        let component = Box::new(MockComponent);
        
        system.add_entity(entity, vec![component]);
        
        assert_eq!(system.entities.len(), 1);
        assert_eq!(system.components.len(), 1);
    }

    #[test]
    fn test_system_remove_entity() {
        let mut system = TestSystem::new();
        let entity = Entity::new();
        let component = Box::new(MockComponent);
        
        system.add_entity(entity, vec![component]);
        system.remove_entity(entity);
        
        assert_eq!(system.entities.len(), 0);
    }
}