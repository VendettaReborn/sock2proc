mod platform;
mod utils;

#[derive(PartialEq)]
pub enum NetworkProtocol {
    TCP = 6,
    UDP = 17,
}
pub use platform::find_process_name;
