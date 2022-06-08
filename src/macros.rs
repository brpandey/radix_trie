macro_rules! enum_extract {
    ($target: expr, $pattern: path) => {
        {
            if let $pattern(v) = $target {
                v
            } else {
                panic!("Pattern not found {}", stringify!($pattern));
            }
        }
    };
}

pub(crate) use enum_extract;
