macro_rules! v8 {
    ($x:expr) => {
        &JsValue::from($x)
    };
}

macro_rules! clone {
    ($x:tt) => {
        let $x = $x.clone();
    };
}
