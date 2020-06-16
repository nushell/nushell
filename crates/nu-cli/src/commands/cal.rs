use crate::commands::{command::EvaluatedWholeStreamCommandArgs, WholeStreamCommand};
use crate::prelude::*;
use chrono::{Datelike, Local, NaiveDate};
use indexmap::IndexMap;
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
            .switch(
                "month-names",
                "Display the month names instead of integers",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Display a calendar."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        cal(args, registry).await
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
        ]
    }
}

pub async fn cal(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let args = args.evaluate_once(&registry).await?;
    let mut calendar_vec_deque = VecDeque::new();
    let tag = args.call_info.name_tag.clone();

    let (current_year, current_month, current_day) = get_current_date();

    let mut selected_year: i32 = current_year;
    let mut current_day_option: Option<u32> = Some(current_day);

    let month_range = if args.has("full-year") {
        if let Some(full_year_value) = args.get("full-year") {
            if let Ok(year_u64) = full_year_value.as_u64() {
                selected_year = year_u64 as i32;

                if selected_year != current_year {
                    current_day_option = None
                }
            } else {
                return Err(get_invalid_year_shell_error(&full_year_value.tag()));
            }
        }

        (1, 12)
    } else {
        (current_month, current_month)
    };

    let add_months_of_year_to_table_result = add_months_of_year_to_table(
        &args,
        &mut calendar_vec_deque,
        &tag,
        selected_year,
        month_range,
        current_month,
        current_day_option,
    );

    match add_months_of_year_to_table_result {
        Ok(()) => Ok(futures::stream::iter(calendar_vec_deque).to_output_stream()),
        Err(error) => Err(error),
    }
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

    let day_limit =
        month_helper.number_of_days_in_month + month_helper.day_number_of_week_month_starts_on;
    let mut day_count: u32 = 1;

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
    let should_show_quarter_column = args.has("quarter");
    let should_show_month_column = args.has("month");
    let should_show_month_names = args.has("month-names");

    while day_count <= day_limit {
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

        if should_show_month_column {
            let month_value = if should_show_month_names {
                UntaggedValue::string(month_helper.month_name.clone()).into_value(tag)
            } else {
                UntaggedValue::int(month_helper.selected_month).into_value(tag)
            };

            indexmap.insert("month".to_string(), month_value);
        }

        for day in &days_of_the_week {
            let should_add_day_number_to_table = (day_count <= day_limit)
                && (day_count > month_helper.day_number_of_week_month_starts_on);

            let mut value = UntaggedValue::nothing().into_value(tag);

            if should_add_day_number_to_table {
                let day_count_with_offset =
                    day_count - month_helper.day_number_of_week_month_starts_on;

                value = UntaggedValue::int(day_count_with_offset).into_value(tag);

                if let Some(current_day) = current_day_option {
                    if current_day == day_count_with_offset {
                        // TODO: Update the value here with a color when color support is added
                        // This colors the current day
                    }
                }
            }

            indexmap.insert((*day).to_string(), value);

            day_count += 1;
        }

        calendar_vec_deque
            .push_back(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(tag));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Cal;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Cal {})
    }
}
