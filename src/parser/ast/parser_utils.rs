crate fn concat<T>(item: T, rest: Vec<T>) -> Vec<T> {
    let mut out = vec![item];
    out.extend(rest);
    out
}
