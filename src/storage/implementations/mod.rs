// Declare the submodules within the implementations directory
pub mod in_memory;
pub mod file_storage;

pub use in_memory::InMemoryStorage;
pub use file_storage::FileStorage;
// pub mod file_storage; // Add this when file_storage.rs is implemented 