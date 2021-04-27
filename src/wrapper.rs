use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "wrapper.js")]
extern "C" {
    pub fn pSBC(p: f64, c0: &str, c1: bool, l: bool) -> JsValue;
    pub fn log(s: String);

    pub fn info_log(s: String);
    pub fn error_log(s: String);
    pub fn success_log(s: String);

    pub fn query_name() -> String;

}

#[macro_export]
macro_rules! do_log {
    ($($arg:tt)*) => ({
        wrapper::log(format!($($arg)*));
    })
}

#[macro_export]
macro_rules! do_error_log {
    ($($arg:tt)*) => ({
        wrapper::error_log(format!($($arg)*));
    })
}

#[macro_export]
macro_rules! do_info_log {
    ($($arg:tt)*) => ({
        wrapper::info_log(format!($($arg)*));
    })
}

#[macro_export]
macro_rules! do_success_log {
    ($($arg:tt)*) => ({
        wrapper::success_log(format!($($arg)*));
    })
}
