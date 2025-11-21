pub mod cli;
pub mod db;
pub mod models;
pub mod services;
pub mod ui;
pub mod utils;

pub mod test_utils;

pub use db::*;
pub use models::*;
pub use utils::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
