#[macro_export]
macro_rules! signature {
    (def $name:tt {
        $usage:tt
        $(
            $positional_name:tt $positional_ty:tt - $positional_desc:tt
        )*
    }) => {{
        let signature = Signature::new(stringify!($name)).desc($usage);
        $(
            $crate::positional! { signature, $positional_name $positional_ty - $positional_desc }
        )*
        signature
    }};
}

#[macro_export]
macro_rules! positional {
    ($ident:tt, $name:tt (optional $shape:tt) - $desc:tt) => {
        let $ident = $ident.optional(stringify!($name), SyntaxShape::$shape, $desc);
    };
    ($ident:tt, $name:tt ($shape:tt)- $desc:tt) => {
        let $ident = $ident.required(stringify!($name), SyntaxShape::$shape, $desc);
    };
}
