#[cfg(crossbeam_loom)]
mod loom {
    #[cfg(test)]
    pub use loom::thread;
    pub use loom::{alloc, sync};
}

#[cfg(crossbeam_loom)]
pub use loom::*;

#[cfg(not(crossbeam_loom))]
mod std {
    #[cfg(test)]
    pub use std::thread;
    pub use std::{alloc, sync};
}
#[cfg(not(crossbeam_loom))]
pub use std::*;
