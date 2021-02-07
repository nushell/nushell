use indexmap::indexmap;
use indexmap::set::IndexSet;
use itertools::Itertools;
use nu_errors::{ExpectedRange, ShellError};
use nu_protocol::{
    ColumnPath, MaybeOwned, PathMember, Primitive, ShellTypeName, SpannedTypeName,
    UnspannedPathMember, UntaggedValue, Value,
};
use nu_source::{
    HasFallibleSpan, HasSpan, PrettyDebug, Span, Spanned, SpannedItem, Tag, Tagged, TaggedItem,
};
use num_traits::cast::ToPrimitive;

pub trait ValueExt {
    fn into_parts(self) -> (UntaggedValue, Tag);
    fn get_data(&self, desc: &str) -> MaybeOwned<'_, Value>;
    fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value>;
    fn get_data_by_member(&self, name: &PathMember) -> Result<Value, ShellError>;
    fn get_data_by_column_path(
        &self,
        path: &ColumnPath,
        callback: Box<dyn FnOnce(&Value, &PathMember, ShellError) -> ShellError>,
    ) -> Result<Value, ShellError>;
    fn swap_data_by_column_path(
        &self,
        path: &ColumnPath,
        callback: Box<dyn FnOnce(&Value) -> Result<Value, ShellError>>,
    ) -> Result<Value, ShellError>;
    fn insert_data_at_path(&self, path: &str, new_value: Value) -> Option<Value>;
    fn insert_data_at_member(
        &mut self,
        member: &PathMember,
        new_value: Value,
    ) -> Result<(), ShellError>;
    fn forgiving_insert_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        new_value: Value,
    ) -> Result<Value, ShellError>;
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
        get_error: Box<dyn FnOnce(&Value, &PathMember, ShellError) -> ShellError>,
    ) -> Result<Value, ShellError> {
        get_data_by_column_path(self, path, get_error)
    }

    fn swap_data_by_column_path(
        &self,
        path: &ColumnPath,
        callback: Box<dyn FnOnce(&Value) -> Result<Value, ShellError>>,
    ) -> Result<Value, ShellError> {
        swap_data_by_column_path(self, path, callback)
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

    fn forgiving_insert_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        new_value: Value,
    ) -> Result<Value, ShellError> {
        forgiving_insert_data_at_column_path(self, split_path, new_value)
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

pub fn get_data_by_column_path<F>(
    value: &Value,
    path: &ColumnPath,
    get_error: F,
) -> Result<Value, ShellError>
where
    F: FnOnce(&Value, &PathMember, ShellError) -> ShellError,
{
    let mut current = value.clone();

    for p in path.iter() {
        let value = get_data_by_member(&current, p);

        match value {
            Ok(v) => current = v.clone(),
            Err(e) => return Err(get_error(&current, &p, e)),
        }
    }

    Ok(current)
}

pub fn swap_data_by_column_path<F>(
    value: &Value,
    path: &ColumnPath,
    callback: F,
) -> Result<Value, ShellError>
where
    F: FnOnce(&Value) -> Result<Value, ShellError>,
{
    let fields = path.clone();

    let to_replace =
        get_data_by_column_path(&value, path, move |obj_source, column_path_tried, error| {
            let path_members_span = fields.maybe_span().unwrap_or_else(Span::unknown);

            match &obj_source.value {
                UntaggedValue::Table(rows) => match column_path_tried {
                    PathMember {
                        unspanned: UnspannedPathMember::String(column),
                        ..
                    } => {
                        let primary_label = format!("There isn't a column named '{}'", &column);

                        let suggestions: IndexSet<_> = rows
                            .iter()
                            .filter_map(|r| {
                                nu_protocol::did_you_mean(&r, column_path_tried.as_string())
                            })
                            .map(|s| s[0].to_owned())
                            .collect();
                        let mut existing_columns: IndexSet<_> = IndexSet::default();
                        let mut names: Vec<String> = vec![];

                        for row in rows {
                            for field in row.data_descriptors() {
                                if !existing_columns.contains(&field[..]) {
                                    existing_columns.insert(field.clone());
                                    names.push(field);
                                }
                            }
                        }

                        if names.is_empty() {
                            return ShellError::labeled_error_with_secondary(
                                "Unknown column",
                                primary_label,
                                column_path_tried.span,
                                "Appears to contain rows. Try indexing instead.",
                                column_path_tried.span.since(path_members_span),
                            );
                        } else {
                            return ShellError::labeled_error_with_secondary(
                                "Unknown column",
                                primary_label,
                                column_path_tried.span,
                                format!(
                                    "Perhaps you meant '{}'? Columns available: {}",
                                    suggestions
                                        .iter()
                                        .map(|x| x.to_owned())
                                        .collect::<Vec<String>>()
                                        .join(","),
                                    names.join(", ")
                                ),
                                column_path_tried.span.since(path_members_span),
                            );
                        };
                    }
                    PathMember {
                        unspanned: UnspannedPathMember::Int(idx),
                        ..
                    } => {
                        let total = rows.len();

                        let secondary_label = if total == 1 {
                            "The table only has 1 row".to_owned()
                        } else {
                            format!("The table only has {} rows (0 to {})", total, total - 1)
                        };

                        return ShellError::labeled_error_with_secondary(
                            "Row not found",
                            format!("There isn't a row indexed at {}", idx),
                            column_path_tried.span,
                            secondary_label,
                            column_path_tried.span.since(path_members_span),
                        );
                    }
                },
                UntaggedValue::Row(columns) => match column_path_tried {
                    PathMember {
                        unspanned: UnspannedPathMember::String(column),
                        ..
                    } => {
                        let primary_label = format!("There isn't a column named '{}'", &column);

                        if let Some(suggestions) =
                            nu_protocol::did_you_mean(&obj_source, column_path_tried.as_string())
                        {
                            return ShellError::labeled_error_with_secondary(
                                "Unknown column",
                                primary_label,
                                column_path_tried.span,
                                format!(
                                    "Perhaps you meant '{}'? Columns available: {}",
                                    suggestions[0],
                                    &obj_source.data_descriptors().join(",")
                                ),
                                column_path_tried.span.since(path_members_span),
                            );
                        }
                    }
                    PathMember {
                        unspanned: UnspannedPathMember::Int(idx),
                        ..
                    } => {
                        return ShellError::labeled_error_with_secondary(
                            "No rows available",
                            format!("A row at '{}' can't be indexed.", &idx),
                            column_path_tried.span,
                            format!(
                                "Appears to contain columns. Columns available: {}",
                                columns.keys().join(",")
                            ),
                            column_path_tried.span.since(path_members_span),
                        )
                    }
                },
                _ => {}
            }

            if let Some(suggestions) =
                nu_protocol::did_you_mean(&obj_source, column_path_tried.as_string())
            {
                return ShellError::labeled_error(
                    "Unknown column",
                    format!("did you mean '{}'?", suggestions[0]),
                    column_path_tried.span.since(path_members_span),
                );
            }

            error
        });

    let old_value = to_replace?;
    let replacement = callback(&old_value)?;

    value
        .replace_data_at_column_path(&path, replacement)
        .ok_or_else(|| {
            ShellError::labeled_error("missing column-path", "missing column-path", value.tag.span)
        })
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
                                new_value.value.into_value(&value.tag),
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

pub fn missing_path_members_by_column_path(value: &Value, path: &ColumnPath) -> Option<usize> {
    let mut current = value.clone();

    for (idx, p) in path.iter().enumerate() {
        if let Ok(value) = get_data_by_member(&current, p) {
            current = value;
        } else {
            return Some(idx);
        }
    }

    None
}

pub fn forgiving_insert_data_at_column_path(
    value: &Value,
    split_path: &ColumnPath,
    new_value: Value,
) -> Result<Value, ShellError> {
    let mut original = value.clone();

    if let Some(missed_at) = missing_path_members_by_column_path(value, split_path) {
        let mut paths = split_path.iter().skip(missed_at + 1).collect::<Vec<_>>();
        paths.reverse();

        let mut candidate = new_value;

        for member in paths.iter() {
            match &member.unspanned {
                UnspannedPathMember::String(column_name) => {
                    candidate =
                        UntaggedValue::row(indexmap! { column_name.into() => candidate.clone()})
                            .into_value(&candidate.tag)
                }
                UnspannedPathMember::Int(int) => {
                    let mut rows = vec![];
                    let size = int.to_usize().unwrap_or(0);

                    for _ in 0..=size {
                        rows.push(
                            UntaggedValue::Primitive(Primitive::Nothing).into_value(&candidate.tag),
                        );
                    }
                    rows.push(candidate.clone());
                    candidate = UntaggedValue::Table(rows).into_value(&candidate.tag);
                }
            }
        }

        let cp = ColumnPath::new(
            split_path
                .iter()
                .cloned()
                .take(split_path.members().len() - missed_at + 1)
                .collect::<Vec<_>>(),
        );

        if missed_at == 0 {
            let current: &mut Value = &mut original;
            insert_data_at_member(current, &cp.members()[0], candidate)?;
            return Ok(original);
        }

        if value
            .get_data_by_column_path(&cp, Box::new(move |_, _, err| err))
            .is_ok()
        {
            return insert_data_at_column_path(&value, &cp, candidate);
        } else if let Some((last, front)) = cp.split_last() {
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

            insert_data_at_member(current, &last, candidate)?;

            return Ok(original);
        } else {
            return Err(ShellError::untagged_runtime_error(
                "Internal error: could not split column path correctly",
            ));
        }
    }

    insert_data_at_column_path(&value, split_path, new_value)
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
            "Internal error: could not split column path correctly",
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
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let s = s.to_string().spanned(value.tag.span);

            Ok(ColumnPath::build(&s).tagged(&value.tag))
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
        UntaggedValue::Primitive(Primitive::Date(dt)) => Ok(dt.format("%Y-%m-%d").to_string()),
        UntaggedValue::Primitive(Primitive::Boolean(x)) => Ok(format!("{}", x)),
        UntaggedValue::Primitive(Primitive::Decimal(x)) => Ok(format!("{}", x)),
        UntaggedValue::Primitive(Primitive::Int(x)) => Ok(format!("{}", x)),
        UntaggedValue::Primitive(Primitive::Filesize(x)) => Ok(format!("{}", x)),
        UntaggedValue::Primitive(Primitive::FilePath(x)) => Ok(format!("{}", x.display())),
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => Ok(path
            .iter()
            .map(|member| match &member.unspanned {
                UnspannedPathMember::String(name) => name.to_string(),
                UnspannedPathMember::Int(n) => format!("{}", n),
            })
            .join(".")),

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
    if index.item >= list.len() {
        if index.item == list.len() {
            list.push(new_value);
            return Ok(());
        }

        let mut idx = list.len();

        loop {
            list.push(UntaggedValue::Primitive(Primitive::Nothing).into_value(&new_value.tag));

            idx += 1;

            if idx == index.item {
                list.push(new_value);
                return Ok(());
            }
        }
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
