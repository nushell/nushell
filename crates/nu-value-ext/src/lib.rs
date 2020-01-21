use itertools::Itertools;
use nu_errors::{ExpectedRange, ShellError};
use nu_protocol::{
    ColumnPath, MaybeOwned, PathMember, Primitive, ShellTypeName, SpannedTypeName,
    UnspannedPathMember, UntaggedValue, Value,
};
use nu_source::{HasSpan, PrettyDebug, Spanned, SpannedItem, Tag, Tagged, TaggedItem};
use num_traits::cast::ToPrimitive;

pub trait ValueExt {
    fn into_parts(self) -> (UntaggedValue, Tag);
    fn get_data(&self, desc: &str) -> MaybeOwned<'_, Value>;
    fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value>;
    fn get_data_by_member(&self, name: &PathMember) -> Result<Value, ShellError>;
    fn get_data_by_column_path(
        &self,
        path: &ColumnPath,
        callback: Box<dyn FnOnce((&Value, &PathMember, ShellError)) -> ShellError>,
    ) -> Result<Value, ShellError>;
    fn insert_data_at_path(&self, path: &str, new_value: Value) -> Option<Value>;
    fn insert_data_at_member(
        &mut self,
        member: &PathMember,
        new_value: Value,
    ) -> Result<(), ShellError>;
    fn insert_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        new_value: Value,
    ) -> Result<Value, ShellError>;
    fn replace_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        replaced_value: Value,
    ) -> Option<Value>;
    fn as_column_path(&self) -> Result<Tagged<ColumnPath>, ShellError>;
    fn as_path_member(&self) -> Result<PathMember, ShellError>;
    fn as_string(&self) -> Result<String, ShellError>;
}

impl ValueExt for Value {
    fn into_parts(self) -> (UntaggedValue, Tag) {
        (self.value, self.tag)
    }

    fn get_data(&self, desc: &str) -> MaybeOwned<'_, Value> {
        get_data(self, desc)
    }

    fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value> {
        get_data_by_key(self, name)
    }

    fn get_data_by_member(&self, name: &PathMember) -> Result<Value, ShellError> {
        get_data_by_member(self, name)
    }

    fn get_data_by_column_path(
        &self,
        path: &ColumnPath,
        callback: Box<dyn FnOnce((&Value, &PathMember, ShellError)) -> ShellError>,
    ) -> Result<Value, ShellError> {
        get_data_by_column_path(self, path, callback)
    }

    fn insert_data_at_path(&self, path: &str, new_value: Value) -> Option<Value> {
        insert_data_at_path(self, path, new_value)
    }

    fn insert_data_at_member(
        &mut self,
        member: &PathMember,
        new_value: Value,
    ) -> Result<(), ShellError> {
        insert_data_at_member(self, member, new_value)
    }

    fn insert_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        new_value: Value,
    ) -> Result<Value, ShellError> {
        insert_data_at_column_path(self, split_path, new_value)
    }

    fn replace_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        replaced_value: Value,
    ) -> Option<Value> {
        replace_data_at_column_path(self, split_path, replaced_value)
    }

    fn as_column_path(&self) -> Result<Tagged<ColumnPath>, ShellError> {
        as_column_path(self)
    }

    fn as_path_member(&self) -> Result<PathMember, ShellError> {
        as_path_member(self)
    }

    fn as_string(&self) -> Result<String, ShellError> {
        as_string(self)
    }
}

pub fn get_data_by_member(value: &Value, name: &PathMember) -> Result<Value, ShellError> {
    match &value.value {
        // If the value is a row, the member is a column name
        UntaggedValue::Row(o) => match &name.unspanned {
            // If the member is a string, get the data
            UnspannedPathMember::String(string) => o
                .get_data_by_key(string[..].spanned(name.span))
                .ok_or_else(|| {
                    ShellError::missing_property(
                        "row".spanned(value.tag.span),
                        string.spanned(name.span),
                    )
                }),

            // If the member is a number, it's an error
            UnspannedPathMember::Int(_) => Err(ShellError::invalid_integer_index(
                "row".spanned(value.tag.span),
                name.span,
            )),
        },

        // If the value is a table
        UntaggedValue::Table(l) => {
            match &name.unspanned {
                // If the member is a string, map over the member
                UnspannedPathMember::String(string) => {
                    let mut out = vec![];

                    for item in l {
                        if let Value {
                            value: UntaggedValue::Row(o),
                            ..
                        } = item
                        {
                            if let Some(v) = o.get_data_by_key(string[..].spanned(name.span)) {
                                out.push(v)
                            }
                        }
                    }

                    if out.is_empty() {
                        Err(ShellError::missing_property(
                            "table".spanned(value.tag.span),
                            string.spanned(name.span),
                        ))
                    } else {
                        Ok(UntaggedValue::Table(out)
                            .into_value(Tag::new(value.anchor(), name.span)))
                    }
                }
                UnspannedPathMember::Int(int) => {
                    let index = int.to_usize().ok_or_else(|| {
                        ShellError::range_error(
                            ExpectedRange::Usize,
                            &"massive integer".spanned(name.span),
                            "indexing",
                        )
                    })?;

                    get_data_by_index(value, index.spanned(value.tag.span)).ok_or_else(|| {
                        ShellError::range_error(0..(l.len()), &int.spanned(name.span), "indexing")
                    })
                }
            }
        }
        other => Err(ShellError::type_error(
            "row or table",
            other.type_name().spanned(value.tag.span),
        )),
    }
}

pub fn get_data_by_column_path(
    value: &Value,
    path: &ColumnPath,
    callback: Box<dyn FnOnce((&Value, &PathMember, ShellError)) -> ShellError>,
) -> Result<Value, ShellError> {
    let mut current = value.clone();

    for p in path.iter() {
        let value = get_data_by_member(&current, p);

        match value {
            Ok(v) => current = v.clone(),
            Err(e) => return Err(callback((&current, &p.clone(), e))),
        }
    }

    Ok(current)
}

pub fn insert_data_at_path(value: &Value, path: &str, new_value: Value) -> Option<Value> {
    let mut new_obj = value.clone();

    let split_path: Vec<_> = path.split('.').collect();

    if let UntaggedValue::Row(ref mut o) = new_obj.value {
        let mut current = o;

        if split_path.len() == 1 {
            // Special case for inserting at the top level
            current
                .entries
                .insert(path.to_string(), new_value.value.into_value(&value.tag));
            return Some(new_obj);
        }

        for idx in 0..split_path.len() {
            match current.entries.get_mut(split_path[idx]) {
                Some(next) => {
                    if idx == (split_path.len() - 2) {
                        if let UntaggedValue::Row(o) = &mut next.value {
                            o.entries.insert(
                                split_path[idx + 1].to_string(),
                                new_value.value.clone().into_value(&value.tag),
                            );
                        }
                        return Some(new_obj.clone());
                    } else {
                        match next.value {
                            UntaggedValue::Row(ref mut o) => {
                                current = o;
                            }
                            _ => return None,
                        }
                    }
                }
                _ => return None,
            }
        }
    }

    None
}

pub fn insert_data_at_member(
    value: &mut Value,
    member: &PathMember,
    new_value: Value,
) -> Result<(), ShellError> {
    match &mut value.value {
        UntaggedValue::Row(dict) => match &member.unspanned {
            UnspannedPathMember::String(key) => {
                dict.insert_data_at_key(key, new_value);
                Ok(())
            }
            UnspannedPathMember::Int(_) => Err(ShellError::type_error(
                "column name",
                "integer".spanned(member.span),
            )),
        },
        UntaggedValue::Table(array) => match &member.unspanned {
            UnspannedPathMember::String(_) => Err(ShellError::type_error(
                "list index",
                "string".spanned(member.span),
            )),
            UnspannedPathMember::Int(int) => {
                let int = int.to_usize().ok_or_else(|| {
                    ShellError::range_error(
                        ExpectedRange::Usize,
                        &"bigger number".spanned(member.span),
                        "inserting into a list",
                    )
                })?;

                insert_data_at_index(array, int.tagged(member.span), new_value)?;
                Ok(())
            }
        },
        other => match &member.unspanned {
            UnspannedPathMember::String(_) => Err(ShellError::type_error(
                "row",
                other.type_name().spanned(value.span()),
            )),
            UnspannedPathMember::Int(_) => Err(ShellError::type_error(
                "table",
                other.type_name().spanned(value.span()),
            )),
        },
    }
}

pub fn insert_data_at_column_path(
    value: &Value,
    split_path: &ColumnPath,
    new_value: Value,
) -> Result<Value, ShellError> {
    if let Some((last, front)) = split_path.split_last() {
        let mut original = value.clone();

        let mut current: &mut Value = &mut original;

        for member in front {
            let type_name = current.spanned_type_name();

            current = get_mut_data_by_member(current, &member).ok_or_else(|| {
                ShellError::missing_property(
                    member.plain_string(std::usize::MAX).spanned(member.span),
                    type_name,
                )
            })?
        }

        insert_data_at_member(current, &last, new_value)?;

        Ok(original)
    } else {
        Err(ShellError::untagged_runtime_error(
            "Internal error: could not split column-path correctly",
        ))
    }
}

pub fn replace_data_at_column_path(
    value: &Value,
    split_path: &ColumnPath,
    replaced_value: Value,
) -> Option<Value> {
    let mut new_obj: Value = value.clone();
    let mut current = &mut new_obj;
    let split_path = split_path.members();

    for idx in 0..split_path.len() {
        match get_mut_data_by_member(current, &split_path[idx]) {
            Some(next) => {
                if idx == (split_path.len() - 1) {
                    *next = replaced_value.value.into_value(&value.tag);
                    return Some(new_obj);
                } else {
                    current = next;
                }
            }
            None => {
                return None;
            }
        }
    }

    None
}

pub fn as_column_path(value: &Value) -> Result<Tagged<ColumnPath>, ShellError> {
    match &value.value {
        UntaggedValue::Table(table) => {
            let mut out: Vec<PathMember> = vec![];

            for item in table {
                out.push(as_path_member(item)?);
            }

            Ok(ColumnPath::new(out).tagged(&value.tag))
        }

        UntaggedValue::Primitive(Primitive::String(s)) => {
            Ok(ColumnPath::new(vec![PathMember::string(s, &value.tag.span)]).tagged(&value.tag))
        }

        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => {
            Ok(path.clone().tagged(value.tag.clone()))
        }

        other => Err(ShellError::type_error(
            "column path",
            other.type_name().spanned(value.span()),
        )),
    }
}

pub fn as_path_member(value: &Value) -> Result<PathMember, ShellError> {
    match &value.value {
        UntaggedValue::Primitive(primitive) => match primitive {
            Primitive::Int(int) => Ok(PathMember::int(int.clone(), value.tag.span)),
            Primitive::String(string) => Ok(PathMember::string(string, value.tag.span)),
            other => Err(ShellError::type_error(
                "path member",
                other.type_name().spanned(value.span()),
            )),
        },
        other => Err(ShellError::type_error(
            "path member",
            other.type_name().spanned(value.span()),
        )),
    }
}

pub fn as_string(value: &Value) -> Result<String, ShellError> {
    match &value.value {
        UntaggedValue::Primitive(Primitive::String(s)) => Ok(s.clone()),
        UntaggedValue::Primitive(Primitive::Boolean(x)) => Ok(format!("{}", x)),
        UntaggedValue::Primitive(Primitive::Decimal(x)) => Ok(format!("{}", x)),
        UntaggedValue::Primitive(Primitive::Int(x)) => Ok(format!("{}", x)),
        UntaggedValue::Primitive(Primitive::Bytes(x)) => Ok(format!("{}", x)),
        UntaggedValue::Primitive(Primitive::Path(x)) => Ok(format!("{}", x.display())),
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => {
            let joined = path
                .iter()
                .map(|member| match &member.unspanned {
                    UnspannedPathMember::String(name) => name.to_string(),
                    UnspannedPathMember::Int(n) => format!("{}", n),
                })
                .join(".");

            if joined.contains(' ') {
                Ok(format!("\"{}\"", joined))
            } else {
                Ok(joined)
            }
        }

        // TODO: this should definitely be more general with better errors
        other => Err(ShellError::labeled_error(
            "Expected string",
            other.type_name(),
            &value.tag,
        )),
    }
}

fn insert_data_at_index(
    list: &mut Vec<Value>,
    index: Tagged<usize>,
    new_value: Value,
) -> Result<(), ShellError> {
    if list.len() >= index.item {
        Err(ShellError::range_error(
            0..(list.len()),
            &format_args!("{}", index.item).spanned(index.tag.span),
            "insert at index",
        ))
    } else {
        list[index.item] = new_value;
        Ok(())
    }
}

pub fn get_data<'value>(value: &'value Value, desc: &str) -> MaybeOwned<'value, Value> {
    match &value.value {
        UntaggedValue::Primitive(_) => MaybeOwned::Borrowed(value),
        UntaggedValue::Row(o) => o.get_data(desc),
        UntaggedValue::Block(_) | UntaggedValue::Table(_) | UntaggedValue::Error(_) => {
            MaybeOwned::Owned(UntaggedValue::nothing().into_untagged_value())
        }
    }
}

pub(crate) fn get_data_by_index(value: &Value, idx: Spanned<usize>) -> Option<Value> {
    match &value.value {
        UntaggedValue::Table(value_set) => {
            let value = value_set.get(idx.item)?;
            Some(
                value
                    .value
                    .clone()
                    .into_value(Tag::new(value.anchor(), idx.span)),
            )
        }
        _ => None,
    }
}

pub fn get_data_by_key(value: &Value, name: Spanned<&str>) -> Option<Value> {
    match &value.value {
        UntaggedValue::Row(o) => o.get_data_by_key(name),
        UntaggedValue::Table(l) => {
            let mut out = vec![];
            for item in l {
                match item {
                    Value {
                        value: UntaggedValue::Row(o),
                        ..
                    } => match o.get_data_by_key(name) {
                        Some(v) => out.push(v),
                        None => out.push(UntaggedValue::nothing().into_untagged_value()),
                    },
                    _ => out.push(UntaggedValue::nothing().into_untagged_value()),
                }
            }

            if !out.is_empty() {
                Some(UntaggedValue::Table(out).into_value(name.span))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn get_mut_data_by_member<'value>(
    value: &'value mut Value,
    name: &PathMember,
) -> Option<&'value mut Value> {
    match &mut value.value {
        UntaggedValue::Row(o) => match &name.unspanned {
            UnspannedPathMember::String(string) => o.get_mut_data_by_key(&string),
            UnspannedPathMember::Int(_) => None,
        },
        UntaggedValue::Table(l) => match &name.unspanned {
            UnspannedPathMember::String(string) => {
                for item in l {
                    if let Value {
                        value: UntaggedValue::Row(o),
                        ..
                    } = item
                    {
                        if let Some(v) = o.get_mut_data_by_key(&string) {
                            return Some(v);
                        }
                    }
                }
                None
            }
            UnspannedPathMember::Int(int) => {
                let index = int.to_usize()?;
                l.get_mut(index)
            }
        },
        _ => None,
    }
}
