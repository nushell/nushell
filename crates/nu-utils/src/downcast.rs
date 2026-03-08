use std::any::Any;

/// On-stack downcasting for specialization in generic functions.
/// ```
/// # use std::any::Any;
/// # use std::fmt::Display;
/// # use nu_utils::downcast;
/// fn foo<T: Display + Any>(x: T) {
///     match downcast::<T, usize>(x) {
///         Ok(x) => println!("usize: {x}"),
///         Err(x) => println!("other: {x}"),
///     }
/// }
/// ```
pub fn downcast<T: Any, Target: Any>(x: T) -> Result<Target, T> {
    let mut x = Some(x);
    let x: &mut dyn Any = &mut x;

    if let Some(target) = x.downcast_mut::<Option<Target>>().and_then(Option::take) {
        Ok(target)
    } else {
        let x = x
            .downcast_mut::<Option<T>>()
            .and_then(Option::take)
            .expect("downcasting to same type can't fail");
        Err(x)
    }
}
