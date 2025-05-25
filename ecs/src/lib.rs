
pub mod components;
pub mod entity;
pub use  macros::component;
pub use components::Component;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}
