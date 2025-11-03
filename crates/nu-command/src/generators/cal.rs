use chrono::{Datelike, Local, NaiveDate};
use nu_color_config::StyleComputer;
use nu_engine::command_prelude::*;
use nu_protocol::ast::{self, Expr, Expression};

use std::collections::VecDeque;

static DAYS_OF_THE_WEEK: [&str; 7] = ["su", "mo", "tu", "we", "th", "fr", "sa"];

#[derive(Clone)]
pub struct Cal;

struct Arguments {
    year: bool,
    quarter: bool,
    month: bool,
    month_names: bool,
    full_year: Option<Spanned<i64>>,
    week_start: Option<Spanned<String>>,
    as_table: bool,
}

impl Command for Cal {
    fn name(&self) -> &str {
        "cal"
    }

    fn signature(&self) -> Signature {
        Signature::build("cal")
            .switch("year", "Display the year column", Some('y'))
            .switch("quarter", "Display the quarter column", Some('q'))
            .switch("month", "Display the month column", Some('m'))
            .switch("as-table", "output as a table", Some('t'))
            .named(
                "full-year",
                SyntaxShape::Int,
                "Display a year-long calendar for the specified year",
                None,
            )
            .param(
                Flag::new("week-start")
                    .arg(SyntaxShape::String)
                    .desc(
                        "Display the calendar with the specified day as the first day of the week",
                    )
                    .completion(Completion::new_list(DAYS_OF_THE_WEEK.as_slice())),
            )
            .switch(
                "month-names",
                "Display the month names instead of integers",
                None,
            )
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::Nothing, Type::table()),
            ])
            .allow_variants_without_examples(true) // TODO: supply exhaustive examples
            .category(Category::Generators)
    }

    fn description(&self) -> &str {
        "Display a calendar."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        cal(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "This month's calendar",
                example: "cal",
                result: None,
            },
            Example {
                description: "The calendar for all of 2012",
                example: "cal --full-year 2012",
                result: None,
            },
            Example {
                description: "This month's calendar with the week starting on Monday",
                example: "cal --week-start mo",
                result: None,
            },
            Example {
                description: "How many 'Friday the Thirteenths' occurred in 2015?",
                example: "cal --as-table --full-year 2015 | where fr == 13 | length",
                result: None,
            },
        ]
    }
}

pub fn cal(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let mut calendar_vec_deque = VecDeque::new();
    let tag = call.head;

    let (current_year, current_month, current_day) = get_current_date();

    let arguments = Arguments {
        year: call.has_flag(engine_state, stack, "year")?,
        month: call.has_flag(engine_state, stack, "month")?,
        month_names: call.has_flag(engine_state, stack, "month-names")?,
        quarter: call.has_flag(engine_state, stack, "quarter")?,
        full_year: call.get_flag(engine_state, stack, "full-year")?,
        week_start: call.get_flag(engine_state, stack, "week-start")?,
        as_table: call.has_flag(engine_state, stack, "as-table")?,
    };

    let style_computer = &StyleComputer::from_config(engine_state, stack);

    let mut selected_year: i32 = current_year;
    let mut current_day_option: Option<u32> = Some(current_day);

    let full_year_value = &arguments.full_year;
    let month_range = if let Some(full_year_value) = full_year_value {
        selected_year = full_year_value.item as i32;

        if selected_year != current_year {
            current_day_option = None
        }
        (1, 12)
    } else {
        (current_month, current_month)
    };

    add_months_of_year_to_table(
        &arguments,
        &mut calendar_vec_deque,
        tag,
        selected_year,
        month_range,
        current_month,
        current_day_option,
        style_computer,
    )?;

    let mut table_no_index = ast::Call::new(Span::unknown());
    table_no_index.add_named((
        Spanned {
            item: "index".to_string(),
            span: Span::unknown(),
        },
        None,
        Some(Expression::new_unknown(
            Expr::Bool(false),
            Span::unknown(),
            Type::Bool,
        )),
    ));

    let cal_table_output =
        Value::list(calendar_vec_deque.into_iter().collect(), tag).into_pipeline_data();
    if !arguments.as_table {
        crate::Table.run(
            engine_state,
            stack,
            &(&table_no_index).into(),
            cal_table_output,
        )
    } else {
        Ok(cal_table_output)
    }
}

fn get_invalid_year_shell_error(head: Span) -> ShellError {
    ShellError::TypeMismatch {
        err_message: "The year is invalid".to_string(),
        span: head,
    }
}

struct MonthHelper {
    selected_year: i32,
    selected_month: u32,
    day_number_of_week_month_starts_on: u32,
    number_of_days_in_month: u32,
    quarter_number: u32,
    month_name: String,
}

impl MonthHelper {
    pub fn new(selected_year: i32, selected_month: u32) -> Result<MonthHelper, ()> {
        let naive_date = NaiveDate::from_ymd_opt(selected_year, selected_month, 1).ok_or(())?;
        let number_of_days_in_month =
            MonthHelper::calculate_number_of_days_in_month(selected_year, selected_month)?;

        Ok(MonthHelper {
            selected_year,
            selected_month,
            day_number_of_week_month_starts_on: naive_date.weekday().num_days_from_sunday(),
            number_of_days_in_month,
            quarter_number: ((selected_month - 1) / 3) + 1,
            month_name: naive_date.format("%B").to_string().to_ascii_lowercase(),
        })
    }

    fn calculate_number_of_days_in_month(
        mut selected_year: i32,
        mut selected_month: u32,
    ) -> Result<u32, ()> {
        // Chrono does not provide a method to output the amount of days in a month
        // This is a workaround taken from the example code from the Chrono docs here:
        // https://docs.rs/chrono/0.3.0/chrono/naive/date/struct.NaiveDate.html#example-30
        if selected_month == 12 {
            selected_year += 1;
            selected_month = 1;
        } else {
            selected_month += 1;
        };

        let next_month_naive_date =
            NaiveDate::from_ymd_opt(selected_year, selected_month, 1).ok_or(())?;

        Ok(next_month_naive_date.pred_opt().unwrap_or_default().day())
    }
}

fn get_current_date() -> (i32, u32, u32) {
    let local_now_date = Local::now().date_naive();

    let current_year: i32 = local_now_date.year();
    let current_month: u32 = local_now_date.month();
    let current_day: u32 = local_now_date.day();

    (current_year, current_month, current_day)
}

#[allow(clippy::too_many_arguments)]
fn add_months_of_year_to_table(
    arguments: &Arguments,
    calendar_vec_deque: &mut VecDeque<Value>,
    tag: Span,
    selected_year: i32,
    (start_month, end_month): (u32, u32),
    current_month: u32,
    current_day_option: Option<u32>,
    style_computer: &StyleComputer,
) -> Result<(), ShellError> {
    for month_number in start_month..=end_month {
        let mut new_current_day_option: Option<u32> = None;

        if let Some(current_day) = current_day_option
            && month_number == current_month
        {
            new_current_day_option = Some(current_day)
        }

        let add_month_to_table_result = add_month_to_table(
            arguments,
            calendar_vec_deque,
            tag,
            selected_year,
            month_number,
            new_current_day_option,
            style_computer,
        );

        add_month_to_table_result?
    }

    Ok(())
}

fn add_month_to_table(
    arguments: &Arguments,
    calendar_vec_deque: &mut VecDeque<Value>,
    tag: Span,
    selected_year: i32,
    current_month: u32,
    current_day_option: Option<u32>,
    style_computer: &StyleComputer,
) -> Result<(), ShellError> {
    let month_helper_result = MonthHelper::new(selected_year, current_month);

    let full_year_value: &Option<Spanned<i64>> = &arguments.full_year;

    let month_helper = match month_helper_result {
        Ok(month_helper) => month_helper,
        Err(()) => match full_year_value {
            Some(x) => return Err(get_invalid_year_shell_error(x.span)),
            None => {
                return Err(ShellError::UnknownOperator {
                    op_token: "Issue parsing command, invalid command".to_string(),
                    span: tag,
                });
            }
        },
    };

    let mut days_of_the_week = DAYS_OF_THE_WEEK;
    let mut total_start_offset: u32 = month_helper.day_number_of_week_month_starts_on;

    if let Some(week_start_day) = &arguments.week_start {
        if let Some(position) = days_of_the_week
            .iter()
            .position(|day| *day == week_start_day.item)
        {
            days_of_the_week.rotate_left(position);
            total_start_offset += (days_of_the_week.len() - position) as u32;
            total_start_offset %= days_of_the_week.len() as u32;
        } else {
            return Err(ShellError::TypeMismatch {
                err_message: format!(
                    "The specified week start day is invalid, expected one of {DAYS_OF_THE_WEEK:?}"
                ),
                span: week_start_day.span,
            });
        }
    };

    let mut day_number: u32 = 1;
    let day_limit: u32 = total_start_offset + month_helper.number_of_days_in_month;

    let should_show_year_column = arguments.year;
    let should_show_quarter_column = arguments.quarter;
    let should_show_month_column = arguments.month;
    let should_show_month_names = arguments.month_names;

    while day_number <= day_limit {
        let mut record = Record::new();

        if should_show_year_column {
            record.insert(
                "year".to_string(),
                Value::int(month_helper.selected_year as i64, tag),
            );
        }

        if should_show_quarter_column {
            record.insert(
                "quarter".to_string(),
                Value::int(month_helper.quarter_number as i64, tag),
            );
        }

        if should_show_month_column || should_show_month_names {
            let month_value = if should_show_month_names {
                Value::string(month_helper.month_name.clone(), tag)
            } else {
                Value::int(month_helper.selected_month as i64, tag)
            };

            record.insert("month".to_string(), month_value);
        }

        for day in &days_of_the_week {
            let should_add_day_number_to_table =
                (day_number > total_start_offset) && (day_number <= day_limit);

            let mut value = Value::nothing(tag);

            if should_add_day_number_to_table {
                let adjusted_day_number = day_number - total_start_offset;

                value = Value::int(adjusted_day_number as i64, tag);

                if let Some(current_day) = current_day_option
                    && current_day == adjusted_day_number
                {
                    // This colors the current day
                    let header_style =
                        style_computer.compute("header", &Value::nothing(Span::unknown()));

                    value = Value::string(
                        header_style
                            .paint(adjusted_day_number.to_string())
                            .to_string(),
                        tag,
                    );
                }
            }

            record.insert((*day).to_string(), value);

            day_number += 1;
        }

        calendar_vec_deque.push_back(Value::record(record, tag))
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Cal {})
    }
}
