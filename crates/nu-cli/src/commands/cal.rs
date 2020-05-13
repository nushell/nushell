use crate::prelude::*;
use chrono::{DateTime, Datelike, Local, NaiveDate};
use nu_errors::ShellError;
use nu_protocol::Dictionary;

use crate::commands::{command::EvaluatedWholeStreamCommandArgs, WholeStreamCommand};
use indexmap::IndexMap;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};

pub struct Cal;

impl WholeStreamCommand for Cal {
    fn name(&self) -> &str {
        "cal"
    }

    fn signature(&self) -> Signature {
        Signature::build("cal")
            .switch("year", "Display the year column", Some('y'))
            .switch("quarter", "Display the quarter column", Some('q'))
            .switch("month", "Display the month column", Some('m'))
            .named(
                "full-year",
                SyntaxShape::Int,
                "Display a year-long calendar for the specified year",
                None,
            )
            .switch(
                "month-names",
                "Display the month names instead of integers",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Display a calendar."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        cal(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "This month's calendar",
                example: "cal",
            },
            Example {
                description: "The calendar for all of 2012",
                example: "cal --full-year 2012",
            },
        ]
    }
}

pub fn cal(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let mut calendar_vec_deque = VecDeque::new();
    let tag = args.call_info.name_tag.clone();

    let (current_year, current_month, current_day) = get_current_date();

    if args.has("full-year") {
        let mut day_value: Option<u32> = Some(current_day);
        let mut year_value = current_year as u64;

        if let Some(year) = args.get("full-year") {
            if let Ok(year_u64) = year.as_u64() {
                year_value = year_u64;
            }

            if year_value != current_year as u64 {
                day_value = None
            }
        }

        add_year_to_table(
            &mut calendar_vec_deque,
            &tag,
            year_value as i32,
            current_year,
            current_month,
            day_value,
            &args,
        );
    } else {
        let (day_start_offset, number_of_days_in_month, _) =
            get_month_information(current_year, current_month, current_year);

        add_month_to_table(
            &mut calendar_vec_deque,
            &tag,
            current_year,
            current_month,
            Some(current_day),
            day_start_offset,
            number_of_days_in_month as usize,
            &args,
        );
    }

    Ok(futures::stream::iter(calendar_vec_deque).to_output_stream())
}

fn get_current_date() -> (i32, u32, u32) {
    let local_now: DateTime<Local> = Local::now();

    let current_year: i32 = local_now.date().year();
    let current_month: u32 = local_now.date().month();
    let current_day: u32 = local_now.date().day();

    (current_year, current_month, current_day)
}

fn add_year_to_table(
    mut calendar_vec_deque: &mut VecDeque<Value>,
    tag: &Tag,
    mut selected_year: i32,
    current_year: i32,
    current_month: u32,
    current_day_option: Option<u32>,
    args: &EvaluatedWholeStreamCommandArgs,
) {
    for month_number in 1..=12 {
        let (day_start_offset, number_of_days_in_month, chosen_date_is_valid) =
            get_month_information(selected_year, month_number, current_year);

        if !chosen_date_is_valid {
            selected_year = current_year;
        }

        let mut new_current_day_option: Option<u32> = None;

        if let Some(current_day) = current_day_option {
            if month_number == current_month {
                new_current_day_option = Some(current_day)
            }
        }

        add_month_to_table(
            &mut calendar_vec_deque,
            &tag,
            selected_year,
            month_number,
            new_current_day_option,
            day_start_offset,
            number_of_days_in_month,
            &args,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn add_month_to_table(
    calendar_vec_deque: &mut VecDeque<Value>,
    tag: &Tag,
    year: i32,
    month: u32,
    _current_day_option: Option<u32>, // Can be used in the future to display current day
    day_start_offset: usize,
    number_of_days_in_month: usize,
    args: &EvaluatedWholeStreamCommandArgs,
) {
    let day_limit = number_of_days_in_month + day_start_offset;
    let mut day_count: usize = 1;

    let days_of_the_week = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
    ];

    let should_show_year_column = args.has("year");
    let should_show_month_column = args.has("month");
    let should_show_quarter_column = args.has("quarter");
    let should_show_month_names = args.has("month-names");

    while day_count <= day_limit {
        let mut indexmap = IndexMap::new();

        if should_show_year_column {
            indexmap.insert("year".to_string(), UntaggedValue::int(year).into_value(tag));
        }

        if should_show_quarter_column {
            indexmap.insert(
                "quarter".to_string(),
                UntaggedValue::int(get_quarter_number(month)).into_value(tag),
            );
        }

        if should_show_month_column {
            let month_value = if should_show_month_names {
                UntaggedValue::string(get_month_name(month)).into_value(tag)
            } else {
                UntaggedValue::int(month).into_value(tag)
            };

            indexmap.insert("month".to_string(), month_value);
        }

        for day in &days_of_the_week {
            let value = if (day_count <= day_limit) && (day_count > day_start_offset) {
                UntaggedValue::int(day_count - day_start_offset).into_value(tag)
            } else {
                UntaggedValue::nothing().into_value(tag)
            };

            indexmap.insert((*day).to_string(), value);

            day_count += 1;
        }

        calendar_vec_deque
            .push_back(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(tag));
    }
}

fn get_quarter_number(month_number: u32) -> u8 {
    match month_number {
        1..=3 => 1,
        4..=6 => 2,
        7..=9 => 3,
        _ => 4,
    }
}

fn get_month_name(month_number: u32) -> String {
    let month_name = match month_number {
        1 => "january",
        2 => "february",
        3 => "march",
        4 => "april",
        5 => "may",
        6 => "june",
        7 => "july",
        8 => "august",
        9 => "september",
        10 => "october",
        11 => "november",
        _ => "december",
    };

    month_name.to_string()
}

fn get_month_information(
    selected_year: i32,
    month: u32,
    current_year: i32,
) -> (usize, usize, bool) {
    let (naive_date, chosen_date_is_valid_one) =
        get_safe_naive_date(selected_year, month, current_year);
    let weekday = naive_date.weekday();
    let (days_in_month, chosen_date_is_valid_two) =
        get_days_in_month(selected_year, month, current_year);

    (
        weekday.num_days_from_sunday() as usize,
        days_in_month,
        chosen_date_is_valid_one && chosen_date_is_valid_two,
    )
}

fn get_days_in_month(selected_year: i32, month: u32, current_year: i32) -> (usize, bool) {
    // Chrono does not provide a method to output the amount of days in a month
    // This is a workaround taken from the example code from the Chrono docs here:
    // https://docs.rs/chrono/0.3.0/chrono/naive/date/struct.NaiveDate.html#example-30
    let (adjusted_year, adjusted_month) = if month == 12 {
        (selected_year + 1, 1)
    } else {
        (selected_year, month + 1)
    };

    let (naive_date, chosen_date_is_valid) =
        get_safe_naive_date(adjusted_year, adjusted_month, current_year);

    (naive_date.pred().day() as usize, chosen_date_is_valid)
}

fn get_safe_naive_date(
    selected_year: i32,
    selected_month: u32,
    current_year: i32,
) -> (NaiveDate, bool) {
    if let Some(naive_date) = NaiveDate::from_ymd_opt(selected_year, selected_month, 1) {
        return (naive_date, true);
    }

    (NaiveDate::from_ymd(current_year, selected_month, 1), false)
}
