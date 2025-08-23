use std::ops::Deref;

pub enum MultiLife<'out, 'local, T>
where
    'out: 'local,
    T: ?Sized,
{
    Out(&'out T),
    Local(&'local T),
}

impl<'out, 'local, T> Deref for MultiLife<'out, 'local, T>
where
    'out: 'local,
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match *self {
            MultiLife::Out(x) => x,
            MultiLife::Local(x) => x,
        }
    }
}
