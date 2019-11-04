use crate::errors::ExpectedRange;
use crate::parser::hir::path::{PathMember, RawPathMember};
use crate::prelude::*;
use crate::ColumnPath;
use crate::SpannedTypeName;

impl Tagged<Value> {
    pub(crate) fn get_data_by_member(
        &self,
        name: &PathMember,
    ) -> Result<Tagged<Value>, ShellError> {
        match &self.item {
            // If the value is a row, the member is a column name
            Value::Row(o) => match &name.item {
                // If the member is a string, get the data
                RawPathMember::String(string) => o
                    .get_data_by_key(string[..].spanned(name.span))
                    .ok_or_else(|| {
                        ShellError::missing_property(
                            "row".spanned(self.tag.span),
                            string.spanned(name.span),
                        )
                    }),

                // If the member is a number, it's an error
                RawPathMember::Int(_) => Err(ShellError::invalid_integer_index(
                    "row".spanned(self.tag.span),
                    name.span,
                )),
            },

            // If the value is a table
            Value::Table(l) => match &name.item {
                // If the member is a string, map over the member
                RawPathMember::String(string) => {
                    let mut out = vec![];

                    for item in l {
                        match item {
                            Tagged {
                                item: Value::Row(o),
                                ..
                            } => match o.get_data_by_key(string[..].spanned(name.span)) {
                                Some(v) => out.push(v),
                                None => {}
                            },
                            _ => {}
                        }
                    }

                    if out.len() == 0 {
                        Err(ShellError::missing_property(
                            "table".spanned(self.tag.span),
                            string.spanned(name.span),
                        ))
                    } else {
                        Ok(Value::Table(out).tagged(Tag::new(self.anchor(), name.span)))
                    }
                }
                RawPathMember::Int(int) => {
                    let index = int.to_usize().ok_or_else(|| {
                        ShellError::range_error(
                            ExpectedRange::Usize,
                            &"massive integer".tagged(name.span),
                            "indexing",
                        )
                    })?;

                    match self.get_data_by_index(index.spanned(self.tag.span)) {
                        Some(v) => Ok(v.clone()),
                        None => Err(ShellError::range_error(
                            0..(l.len()),
                            &int.tagged(name.span),
                            "indexing",
                        )),
                    }
                }
            },
            other => Err(ShellError::type_error(
                "row or table",
                other.spanned(self.tag.span).spanned_type_name(),
            )),
        }
    }

    pub fn get_data_by_column_path(
        &self,
        path: &ColumnPath,
        callback: Box<dyn FnOnce((&Value, &PathMember, ShellError)) -> ShellError>,
    ) -> Result<Tagged<Value>, ShellError> {
        let mut current = self.clone();

        for p in path.iter() {
            let value = current.get_data_by_member(p);

            match value {
                Ok(v) => current = v.clone(),
                Err(e) => return Err(callback((&current.clone(), &p.clone(), e))),
            }
        }

        Ok(current)
    }

    pub fn insert_data_at_path(&self, path: &str, new_value: Value) -> Option<Tagged<Value>> {
        let mut new_obj = self.clone();

        let split_path: Vec<_> = path.split(".").collect();

        if let Value::Row(ref mut o) = new_obj.item {
            let mut current = o;

            if split_path.len() == 1 {
                // Special case for inserting at the top level
                current
                    .entries
                    .insert(path.to_string(), new_value.tagged(&self.tag));
                return Some(new_obj);
            }

            for idx in 0..split_path.len() {
                match current.entries.get_mut(split_path[idx]) {
                    Some(next) => {
                        if idx == (split_path.len() - 2) {
                            match &mut next.item {
                                Value::Row(o) => {
                                    o.entries.insert(
                                        split_path[idx + 1].to_string(),
                                        new_value.tagged(&self.tag),
                                    );
                                }
                                _ => {}
                            }

                            return Some(new_obj.clone());
                        } else {
                            match next.item {
                                Value::Row(ref mut o) => {
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
        &mut self,
        member: &PathMember,
        new_value: Tagged<Value>,
    ) -> Result<(), ShellError> {
        match &mut self.item {
            Value::Row(dict) => match &member.item {
                RawPathMember::String(key) => Ok({
                    dict.insert_data_at_key(key, new_value);
                }),
                RawPathMember::Int(_) => Err(ShellError::type_error(
                    "column name",
                    "integer".spanned(member.span),
                )),
            },
            Value::Table(array) => match &member.item {
                RawPathMember::String(_) => Err(ShellError::type_error(
                    "list index",
                    "string".spanned(member.span),
                )),
                RawPathMember::Int(int) => Ok({
                    let int = int.to_usize().ok_or_else(|| {
                        ShellError::range_error(
                            ExpectedRange::Usize,
                            &"bigger number".tagged(member.span),
                            "inserting into a list",
                        )
                    })?;

                    insert_data_at_index(array, int.tagged(member.span), new_value.clone())?;
                }),
            },
            other => match &member.item {
                RawPathMember::String(_) => Err(ShellError::type_error(
                    "row",
                    other.type_name().spanned(self.span()),
                )),
                RawPathMember::Int(_) => Err(ShellError::type_error(
                    "table",
                    other.type_name().spanned(self.span()),
                )),
            },
        }
    }

    pub fn insert_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        new_value: Tagged<Value>,
    ) -> Result<Tagged<Value>, ShellError> {
        let (last, front) = split_path.split_last();
        let mut original = self.clone();

        let mut current: &mut Tagged<Value> = &mut original;

        for member in front {
            let type_name = current.spanned_type_name();

            current = current
                .item
                .get_mut_data_by_member(&member)
                .ok_or_else(|| {
                    ShellError::missing_property(
                        member.plain_string(std::usize::MAX).spanned(member.span),
                        type_name,
                    )
                })?
        }

        current.insert_data_at_member(&last, new_value)?;

        Ok(original)
    }

    pub fn replace_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        replaced_value: Value,
    ) -> Option<Tagged<Value>> {
        let mut new_obj: Tagged<Value> = self.clone();
        let mut current = &mut new_obj;
        let split_path = split_path.members();

        for idx in 0..split_path.len() {
            match current.item.get_mut_data_by_member(&split_path[idx]) {
                Some(next) => {
                    if idx == (split_path.len() - 1) {
                        *next = replaced_value.tagged(&self.tag);
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

    pub fn as_column_path(&self) -> Result<Tagged<ColumnPath>, ShellError> {
        match &self.item {
            Value::Table(table) => {
                let mut out: Vec<PathMember> = vec![];

                for item in table {
                    out.push(item.as_path_member()?);
                }

                Ok(ColumnPath::new(out).tagged(&self.tag))
            }

            Value::Primitive(Primitive::ColumnPath(path)) => {
                Ok(path.clone().tagged(self.tag.clone()))
            }

            other => Err(ShellError::type_error(
                "column path",
                other.type_name().spanned(self.span()),
            )),
        }
    }

    pub fn as_path_member(&self) -> Result<PathMember, ShellError> {
        match &self.item {
            Value::Primitive(primitive) => match primitive {
                Primitive::Int(int) => Ok(PathMember::int(int.clone(), self.tag.span)),
                Primitive::String(string) => Ok(PathMember::string(string, self.tag.span)),
                other => Err(ShellError::type_error(
                    "path member",
                    other.type_name().spanned(self.span()),
                )),
            },
            other => Err(ShellError::type_error(
                "path member",
                other.type_name().spanned(self.span()),
            )),
        }
    }

    pub fn as_string(&self) -> Result<String, ShellError> {
        match &self.item {
            Value::Primitive(Primitive::String(s)) => Ok(s.clone()),
            Value::Primitive(Primitive::Boolean(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Decimal(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Int(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Bytes(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Path(x)) => Ok(format!("{}", x.display())),
            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::labeled_error(
                "Expected string",
                other.type_name(),
                &self.tag,
            )),
        }
    }
}

fn insert_data_at_index(
    list: &mut Vec<Tagged<Value>>,
    index: Tagged<usize>,
    new_value: Tagged<Value>,
) -> Result<(), ShellError> {
    if list.len() >= index.item {
        Err(ShellError::range_error(
            0..(list.len()),
            &format_args!("{}", index.item).tagged(index.tag.clone()),
            "insert at index",
        ))
    } else {
        list[index.item] = new_value;
        Ok(())
    }
}

impl Value {
    pub(crate) fn get_data_by_index(&self, idx: Spanned<usize>) -> Option<Tagged<Value>> {
        match self {
            Value::Table(value_set) => {
                let value = value_set.get(idx.item)?;
                Some(
                    value
                        .item
                        .clone()
                        .tagged(Tag::new(value.anchor(), idx.span)),
                )
            }
            _ => None,
        }
    }

    pub(crate) fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Tagged<Value>> {
        match self {
            Value::Row(o) => o.get_data_by_key(name),
            Value::Table(l) => {
                let mut out = vec![];
                for item in l {
                    match item {
                        Tagged {
                            item: Value::Row(o),
                            ..
                        } => match o.get_data_by_key(name) {
                            Some(v) => out.push(v),
                            None => out.push(Value::nothing().tagged_unknown()),
                        },
                        _ => out.push(Value::nothing().tagged_unknown()),
                    }
                }

                if out.len() > 0 {
                    Some(Value::Table(out).tagged(name.span))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn get_mut_data_by_member(
        &mut self,
        name: &PathMember,
    ) -> Option<&mut Tagged<Value>> {
        match self {
            Value::Row(o) => match &name.item {
                RawPathMember::String(string) => o.get_mut_data_by_key(&string),
                RawPathMember::Int(_) => None,
            },
            Value::Table(l) => match &name.item {
                RawPathMember::String(string) => {
                    for item in l {
                        match item {
                            Tagged {
                                item: Value::Row(o),
                                ..
                            } => match o.get_mut_data_by_key(&string) {
                                Some(v) => return Some(v),
                                None => {}
                            },
                            _ => {}
                        }
                    }
                    None
                }
                RawPathMember::Int(int) => {
                    let index = int.to_usize()?;
                    l.get_mut(index)
                }
            },
            _ => None,
        }
    }
}
