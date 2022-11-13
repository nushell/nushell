use crate::dataframe::eager::sql_expr::parse_sql_expr;
use polars::error::{ErrString, PolarsError};
use polars::prelude::{col, DataFrame, DataType, IntoLazy, LazyFrame};
use sqlparser::ast::{
    Expr as SqlExpr, Select, SelectItem, SetExpr, Statement, TableFactor, Value as SQLValue,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;

#[derive(Default)]
pub struct SQLContext {
    table_map: HashMap<String, LazyFrame>,
    dialect: GenericDialect,
}

impl SQLContext {
    pub fn new() -> Self {
        Self {
            table_map: HashMap::new(),
            dialect: GenericDialect::default(),
        }
    }

    pub fn register(&mut self, name: &str, df: &DataFrame) {
        self.table_map.insert(name.to_owned(), df.clone().lazy());
    }

    fn execute_select(&self, select_stmt: &Select) -> Result<LazyFrame, PolarsError> {
        // Determine involved dataframe
        // Implicit join require some more work in query parsers, Explicit join are preferred for now.
        let tbl = select_stmt.from.get(0).ok_or_else(|| {
            PolarsError::NotFound(ErrString::from("No table found in select statement"))
        })?;
        let mut alias_map = HashMap::new();
        let tbl_name = match &tbl.relation {
            TableFactor::Table { name, alias, .. } => {
                let tbl_name = name
                    .0
                    .get(0)
                    .ok_or_else(|| {
                        PolarsError::NotFound(ErrString::from("No table found in select statement"))
                    })?
                    .value
                    .to_string();
                if self.table_map.contains_key(&tbl_name) {
                    if let Some(alias) = alias {
                        alias_map.insert(alias.name.value.clone(), tbl_name.to_owned());
                    };
                    tbl_name
                } else {
                    return Err(PolarsError::ComputeError(
                        format!("Table name {tbl_name} was not found").into(),
                    ));
                }
            }
            // Support bare table, optional with alias for now
            _ => return Err(PolarsError::ComputeError("Not implemented".into())),
        };
        let df = &self.table_map[&tbl_name];
        let mut raw_projection_before_alias: HashMap<String, usize> = HashMap::new();
        let mut contain_wildcard = false;
        // Filter Expression
        let df = match select_stmt.selection.as_ref() {
            Some(expr) => {
                let filter_expression = parse_sql_expr(expr)?;
                df.clone().filter(filter_expression)
            }
            None => df.clone(),
        };
        // Column Projections
        let projection = select_stmt
            .projection
            .iter()
            .enumerate()
            .map(|(i, select_item)| {
                Ok(match select_item {
                    SelectItem::UnnamedExpr(expr) => {
                        let expr = parse_sql_expr(expr)?;
                        raw_projection_before_alias.insert(format!("{:?}", expr), i);
                        expr
                    }
                    SelectItem::ExprWithAlias { expr, alias } => {
                        let expr = parse_sql_expr(expr)?;
                        raw_projection_before_alias.insert(format!("{:?}", expr), i);
                        expr.alias(&alias.value)
                    }
                    SelectItem::QualifiedWildcard(_) | SelectItem::Wildcard => {
                        contain_wildcard = true;
                        col("*")
                    }
                })
            })
            .collect::<Result<Vec<_>, PolarsError>>()?;
        // Check for group by
        // After projection since there might be number.
        let group_by = select_stmt
            .group_by
            .iter()
            .map(
                |e|match e {
                  SqlExpr::Value(SQLValue::Number(idx, _)) => {
                    let idx = match idx.parse::<usize>() {
                        Ok(0)| Err(_) => Err(
                        PolarsError::ComputeError(
                            format!("Group-By Error: Only positive number or expression are supported, got {idx}").into()
                        )),
                        Ok(idx) => Ok(idx)
                    }?;
                    Ok(projection[idx].clone())
                  }
                  SqlExpr::Value(_) => Err(
                      PolarsError::ComputeError("Group-By Error: Only positive number or expression are supported".into())
                  ),
                  _ => parse_sql_expr(e)
                }
            )
            .collect::<Result<Vec<_>, PolarsError>>()?;

        let df = if group_by.is_empty() {
            df.select(projection)
        } else {
            // check groupby and projection due to difference between SQL and polars
            // Return error on wild card, shouldn't process this
            if contain_wildcard {
                return Err(PolarsError::ComputeError(
                    "Group-By Error: Can't process wildcard in group-by".into(),
                ));
            }
            // Default polars group by will have group by columns at the front
            // need some container to contain position of group by columns and its position
            // at the final agg projection, check the schema for the existence of group by column
            // and its projections columns, keeping the original index
            let (exclude_expr, groupby_pos): (Vec<_>, Vec<_>) = group_by
                .iter()
                .map(|expr| raw_projection_before_alias.get(&format!("{:?}", expr)))
                .enumerate()
                .filter(|(_, proj_p)| proj_p.is_some())
                .map(|(gb_p, proj_p)| (*proj_p.unwrap_or(&0), (*proj_p.unwrap_or(&0), gb_p)))
                .unzip();
            let (agg_projection, agg_proj_pos): (Vec<_>, Vec<_>) = projection
                .iter()
                .enumerate()
                .filter(|(i, _)| !exclude_expr.contains(i))
                .enumerate()
                .map(|(agg_pj, (proj_p, expr))| (expr.clone(), (proj_p, agg_pj + group_by.len())))
                .unzip();
            let agg_df = df.groupby(group_by).agg(agg_projection);
            let mut final_proj_pos = groupby_pos
                .into_iter()
                .chain(agg_proj_pos.into_iter())
                .collect::<Vec<_>>();

            final_proj_pos.sort_by(|(proj_pa, _), (proj_pb, _)| proj_pa.cmp(proj_pb));
            let final_proj = final_proj_pos
                .into_iter()
                .map(|(_, shm_p)| {
                    col(agg_df
                        .clone()
                        // FIXME: had to do this mess to get get_index to work, not sure why. need help
                        .collect()
                        .unwrap_or_default()
                        .schema()
                        .get_index(shm_p)
                        .unwrap_or((&"".to_string(), &DataType::Null))
                        .0)
                })
                .collect::<Vec<_>>();
            agg_df.select(final_proj)
        };
        Ok(df)
    }

    pub fn execute(&self, query: &str) -> Result<LazyFrame, PolarsError> {
        let ast = Parser::parse_sql(&self.dialect, query)
            .map_err(|e| PolarsError::ComputeError(format!("{:?}", e).into()))?;
        if ast.len() != 1 {
            Err(PolarsError::ComputeError(
                "One and only one statement at a time please".into(),
            ))
        } else {
            let ast = ast
                .get(0)
                .ok_or_else(|| PolarsError::NotFound(ErrString::from("No statement found")))?;
            Ok(match ast {
                Statement::Query(query) => {
                    let rs = match &*query.body {
                        SetExpr::Select(select_stmt) => self.execute_select(select_stmt)?,
                        _ => {
                            return Err(PolarsError::ComputeError(
                                "INSERT, UPDATE is not supported for polars".into(),
                            ))
                        }
                    };
                    match &query.limit {
                        Some(SqlExpr::Value(SQLValue::Number(nrow, _))) => {
                            let nrow = nrow.parse().map_err(|err| {
                                PolarsError::ComputeError(
                                    format!("Conversion Error: {:?}", err).into(),
                                )
                            })?;
                            rs.limit(nrow)
                        }
                        None => rs,
                        _ => {
                            return Err(PolarsError::ComputeError(
                                "Only support number argument to LIMIT clause".into(),
                            ))
                        }
                    }
                }
                _ => {
                    return Err(PolarsError::ComputeError(
                        format!("Statement type {:?} is not supported", ast).into(),
                    ))
                }
            })
        }
    }
}
