use crate::prelude::*;
use chrono::{Datelike, Local, NaiveDate};
use indexmap::IndexMap;
use nu_engine::{EvaluatedWholeStreamCommandArgs, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Cal;

#[async_trait]
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
    }

    fn usage(&self) -> &str {
        "Display a calendar."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        cal(args).await
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
                description: "This month's calendar with the week starting on monday",
                example: "cal --week-start monday",
                result: None,
            },
        ]
    }
}

pub async fn cal(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let mut calendar_vec_deque = VecDeque::new();
    let tag = args.call_info.name_tag.clone();

    let (current_year, current_month, current_day) = get_current_date();

    let mut selected_year: i32 = current_year;
    let mut current_day_option: Option<u32> = Some(current_day);

    let month_range = if let Some(full_year_value) = args.get("full-year") {
        if let Ok(year_u64) = full_year_value.as_u64() {
            selected_year = year_u64 as i32;

            if selected_year != current_year {
                current_day_option = None
            }
        } else {
            return Err(get_invalid_year_shell_error(&full_year_value.tag()));
        }

        (1, 12)
    } else {
        (current_month, current_month)
    };

    add_months_of_year_to_table(
        &args,
        &mut calendar_vec_deque,
        &tag,
        selected_year,
        month_range,
        current_month,
        current_day_option,
    )?;

    Ok(futures::stream::iter(calendar_vec_deque).to_output_stream())
}

fn get_invalid_year_shell_error(year_tag: &Tag) -> ShellError {
    ShellError::labeled_error("The year is invalid", "invalid year", year_tag)
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

        Ok(next_month_naive_date.pred().day())
    }
}

fn get_current_date() -> (i32, u32, u32) {
    let local_now_date = Local::now().date();

    let current_year: i32 = local_now_date.year();
    let current_month: u32 = local_now_date.month();
    let current_day: u32 = local_now_date.day();

    (current_year, current_month, current_day)
}

fn add_months_of_year_to_table(
    args: &EvaluatedWholeStreamCommandArgs,
    mut calendar_vec_deque: &mut VecDeque<Value>,
    tag: &Tag,
    selected_year: i32,
    (start_month, end_month): (u32, u32),
    current_month: u32,
    current_day_option: Option<u32>,
) -> Result<(), ShellError> {
    for month_number in start_month..=end_month {
        let mut new_current_day_option: Option<u32> = None;

        if let Some(current_day) = current_day_option {
            if month_number == current_month {
                new_current_day_option = Some(current_day)
            }
        }

        let add_month_to_table_result = add_month_to_table(
            &args,
            &mut calendar_vec_deque,
            &tag,
            selected_year,
            month_number,
            new_current_day_option,
        );

        add_month_to_table_result?
    }

    Ok(())
}

fn add_month_to_table(
    args: &EvaluatedWholeStreamCommandArgs,
    calendar_vec_deque: &mut VecDeque<Value>,
    tag: &Tag,
    selected_year: i32,
    current_month: u32,
    current_day_option: Option<u32>,
) -> Result<(), ShellError> {
    let month_helper_result = MonthHelper::new(selected_year, current_month);

    let month_helper = match month_helper_result {
        Ok(month_helper) => month_helper,
        Err(()) => match args.get("full-year") {
            Some(full_year_value) => {
                return Err(get_invalid_year_shell_error(&full_year_value.tag()))
            }
            None => {
                return Err(ShellError::labeled_error(
                    "Issue parsing command",
                    "invalid command",
                    tag,
                ))
            }
        },
    };

    let mut days_of_the_week = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
    ];

    let mut week_start_day = days_of_the_week[0].to_string();

    if let Some(week_start_value) = args.get("week-start") {
        if let Ok(day) = week_start_value.as_string() {
            if days_of_the_week.contains(&day.as_str()) {
                week_start_day = day;
            } else {
                return Err(ShellError::labeled_error(
                    "The specified week start day is invalid",
                    "invalid week start day",
                    week_start_value.tag(),
                ));
            }
        }
    }

    let week_start_day_offset = days_of_the_week.len()
        - days_of_the_week
            .iter()
            .position(|day| *day == week_start_day)
            .unwrap_or(0);

    days_of_the_week.rotate_right(week_start_day_offset);

    let mut total_start_offset: u32 =
        month_helper.day_number_of_week_month_starts_on + week_start_day_offset as u32;
    total_start_offset %= days_of_the_week.len() as u32;

    let mut day_number: u32 = 1;
    let day_limit: u32 = total_start_offset + month_helper.number_of_days_in_month;

    let should_show_year_column = args.has("year");
    let should_show_quarter_column = args.has("quarter");
    let should_show_month_column = args.has("month");
    let should_show_month_names = args.has("month-names");

    while day_number <= day_limit {
        let mut indexmap = IndexMap::new();

        if should_show_year_column {
            indexmap.insert(
                "year".to_string(),
                UntaggedValue::int(month_helper.selected_year).into_value(tag),
            );
        }

        if should_show_quarter_column {
            indexmap.insert(
                "quarter".to_string(),
                UntaggedValue::int(month_helper.quarter_number).into_value(tag),
            );
        }

        if should_show_month_column || should_show_month_names {
            let month_value = if should_show_month_names {
                UntaggedValue::string(month_helper.month_name.clone()).into_value(tag)
            } else {
                UntaggedValue::int(month_helper.selected_month).into_value(tag)
            };

            indexmap.insert("month".to_string(), month_value);
        }

        for day in &days_of_the_week {
            let should_add_day_number_to_table =
                (day_number > total_start_offset) && (day_number <= day_limit);

            let mut value = UntaggedValue::nothing().into_value(tag);

            if should_add_day_number_to_table {
                let adjusted_day_number = day_number - total_start_offset;

                value = UntaggedValue::int(adjusted_day_number).into_value(tag);

                if let Some(current_day) = current_day_option {
                    if current_day == adjusted_day_number {
                        // TODO: Update the value here with a color when color support is added
                        // This colors the current day
                    }
                }
            }

            indexmap.insert((*day).to_string(), value);

            day_number += 1;
        }

        calendar_vec_deque
            .push_back(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(tag));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Cal;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Cal {})
    }
}
