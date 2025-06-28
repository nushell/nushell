use chrono::{Datelike, Local, NaiveDate};
use nu_color_config::StyleComputer;
use nu_engine::command_prelude::*;
use nu_protocol::ast::{self, Expr, Expression};

use std::collections::VecDeque;

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
            .named(
                "week-start",
                SyntaxShape::String,
                "Display the calendar with the specified day as the first day of the week",
                None,
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

    fn examples(&self) -> Vec<Example> {
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

#[derive(PartialEq)]
enum CalendarType {
    Julian,
    Gregorian,
}

fn get_calendar_type(year: i32, month: u32, day: u32) -> CalendarType {
    // The British Empire adopted the Gregorian calendar in September 1752
    // September 2, 1752 (Julian) was followed by September 14, 1752 (Gregorian)
    match year.cmp(&1752) {
        std::cmp::Ordering::Less => CalendarType::Julian,
        std::cmp::Ordering::Greater => CalendarType::Gregorian,
        std::cmp::Ordering::Equal => match month.cmp(&9) {
            std::cmp::Ordering::Less => CalendarType::Julian,
            std::cmp::Ordering::Greater => CalendarType::Gregorian,
            std::cmp::Ordering::Equal => {
                if day <= 2 {
                    CalendarType::Julian
                } else if day >= 14 {
                    CalendarType::Gregorian
                } else {
                    // Days 3-13 do not exist in the British calendar
                    // We'll treat them as Gregorian for safety, but they should not appear
                    CalendarType::Gregorian
                }
            }
        },
    }
}

fn is_julian_leap_year(year: i32) -> bool {
    year % 4 == 0
}

fn is_gregorian_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn julian_weekday(year: i32, month: u32, day: u32) -> u32 {
    // Julian calendar weekday algorithm
    // Returns 0 for Sunday through 6 for Saturday
    let a = (14 - month as i32) / 12;
    let y = year - a;
    let m = month as i32 + 12 * a - 2;

    let w = (day as i32 + ((31 * m) / 12) + y + (y / 4)) % 7;
    w as u32
}

fn gregorian_weekday(year: i32, month: u32, day: u32) -> u32 {
    // Zeller's congruence for Gregorian calendar
    // Returns 0 for Sunday through 6 for Saturday
    let adjusted_year = if month <= 2 { year - 1 } else { year };
    let adjusted_month = if month <= 2 { month + 12 } else { month };

    let k = day as i32;
    let j = adjusted_year / 100;
    let h = adjusted_year % 100;

    let w = (k + ((13 * (adjusted_month as i32 + 1)) / 5) + h + (h / 4) + (j / 4) - (2 * j)) % 7;
    ((w + 7) % 7) as u32
}

fn get_weekday(year: i32, month: u32, day: u32) -> u32 {
    match get_calendar_type(year, month, day) {
        CalendarType::Julian => julian_weekday(year, month, day),
        CalendarType::Gregorian => gregorian_weekday(year, month, day),
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
    pub fn new(selected_year: i32, selected_month: u32) -> Result<MonthHelper, ShellError> {
        if !matches!(selected_month, 1..=12) {
            return Err(ShellError::TypeMismatch {
                err_message: format!("Invalid month: {selected_month}"),
                span: Span::unknown(),
            });
        }

        // Special case: September 1752 calendar reform (only days 1-2, 14-30 exist)
        let number_of_days_in_month = if selected_year == 1752 && selected_month == 9 {
            19
        } else {
            match get_calendar_type(selected_year, selected_month, 1) {
                CalendarType::Julian => get_julian_days_in_month(selected_month, selected_year),
                CalendarType::Gregorian => {
                    get_gregorian_days_in_month(selected_month, selected_year)
                }
            }
        };

        let day_number_of_week_month_starts_on = get_weekday(selected_year, selected_month, 1);
        let quarter_number = ((selected_month - 1) / 3) + 1;
        let month_name = NaiveDate::from_ymd_opt(selected_year, selected_month, 1)
            .map(|d| d.format("%B").to_string().to_ascii_lowercase())
            .unwrap_or_else(|| "invalid".to_string());

        Ok(MonthHelper {
            selected_year,
            selected_month,
            day_number_of_week_month_starts_on,
            number_of_days_in_month,
            quarter_number,
            month_name,
        })
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

        if let Some(current_day) = current_day_option {
            if month_number == current_month {
                new_current_day_option = Some(current_day)
            }
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
    let month_helper = MonthHelper::new(selected_year, current_month)?;

    let mut days_of_the_week = ["su", "mo", "tu", "we", "th", "fr", "sa"];
    let mut total_start_offset: u32 = month_helper.day_number_of_week_month_starts_on;

    // Handle --week-start flag
    if let Some(week_start_day) = &arguments.week_start {
        if let Some(position) = days_of_the_week
            .iter()
            .position(|day| *day == week_start_day.item)
        {
            days_of_the_week.rotate_left(position);
            // Calculate offset so the first day of the month appears in the correct column
            let offset = (7 + month_helper.day_number_of_week_month_starts_on as i32 - position as i32) % 7;
            total_start_offset = offset as u32;
        } else {
            return Err(ShellError::TypeMismatch {
                err_message: "The specified week start day is invalid".to_string(),
                span: week_start_day.span,
            });
        }
    }

    let mut calendar_days: Vec<Option<i64>> = vec![];

    // 1. Fill in blank days at the start of the month
    for _ in 0..total_start_offset {
        calendar_days.push(None);
    }

    // 2. Fill in the actual days for the month
    if month_helper.selected_year == 1752 && month_helper.selected_month == 9 {
        // Special case logic for the Gregorian cutover - days 1-2 then 14-30
        // The first day of the month is 1, so we need to start at the correct offset
        // Fill nulls up to the weekday of the 1st, then push 1 and 2, then nulls for skipped days, then 14-30
        // Already handled by the offset logic above
        calendar_days.push(Some(1));
        calendar_days.push(Some(2));
        // Skip days 3-13 (inclusive) due to calendar reform
        (14..=30).for_each(|day| calendar_days.push(Some(day as i64)));
    } else {
        // Logic for a normal month
        for day in 1..=month_helper.number_of_days_in_month {
            calendar_days.push(Some(day as i64));
        }
    }

    let should_show_year_column = arguments.year;
    let should_show_quarter_column = arguments.quarter;
    let should_show_month_column = arguments.month;
    let should_show_month_names = arguments.month_names;

    // 3. Create weekly records from the flat list of days
    for week_chunk in calendar_days.chunks(7) {
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

        for (i, day_val_opt) in week_chunk.iter().enumerate() {
            let day_name = days_of_the_week[i];
            let value = match day_val_opt {
                Some(day_val) => {
                    // Color the current day if it matches
                    if let Some(current_day) = current_day_option {
                        if current_day as i64 == *day_val {
                            let header_style =
                                style_computer.compute("header", &Value::nothing(Span::unknown()));
                            Value::string(header_style.paint(day_val.to_string()).to_string(), tag)
                        } else {
                            Value::int(*day_val, tag)
                        }
                    } else {
                        Value::int(*day_val, tag)
                    }
                }
                None => Value::nothing(tag),
            };
            record.insert(day_name.to_string(), value);
        }

        // If the chunk is smaller than 7, fill the rest with null
        if week_chunk.len() < 7 {
            for day_name in days_of_the_week.iter().skip(week_chunk.len()) {
                record.insert((*day_name).to_string(), Value::nothing(tag));
            }
        }

        calendar_vec_deque.push_back(Value::record(record, tag));
    }

    Ok(())
}

fn get_julian_days_in_month(month: u32, year: i32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_julian_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn get_gregorian_days_in_month(month: u32, year: i32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_gregorian_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

#[allow(dead_code)]
fn get_invalid_year_shell_error(head: Span) -> ShellError {
    ShellError::TypeMismatch {
        err_message: "The year is invalid".to_string(),
        span: head,
    }
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
