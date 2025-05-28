use std::{sync::atomic::{AtomicU32, Ordering}};



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] 
pub struct Entity(u32); 

impl Entity{
    pub fn new() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Entity(id)
    }
}

