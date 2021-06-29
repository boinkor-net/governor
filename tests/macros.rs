#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! tests {
    ($($test:item)*) => { $(#[test] $test)* };
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! wait {
    ($expr:expr) => {
        block_on($expr)
    };
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! tests {
    ($($(#[$meta:meta])? fn $name:ident() $body:block)*) => {
        #[cfg(target_arch = "wasm32")]
        use wasm_bindgen_test::wasm_bindgen_test as test;

        $(
            #[test]
            $(#[$meta])?
            async fn $name()
            $body
        )*
    };
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! wait {
    ($expr:expr) => {
        $expr.await
    };
}
